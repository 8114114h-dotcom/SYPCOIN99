// node/lifecycle.rs — Node startup and shutdown sequences.
//
// Startup sequence (order matters — each step depends on the previous):
//   1. Load genesis config
//   2. Open storage
//   3. Load or build genesis state + block
//   4. Build blockchain from genesis
//   5. Restore latest state snapshot from storage
//   6. Build executor from state
//   7. Init mempool
//   8. Init network node
//   9. Init event bus
//  10. Init metrics + security
//
// Shutdown sequence:
//   1. Emit NodeStopping event
//   2. Stop mining loop
//   3. Save current state snapshot
//   4. Flush mempool (log pending count)
//   5. Close storage
//   6. Exit

use config::{NodeConfig, DbBackend};
use consensus::Blockchain;
use event_bus::{ChainEvent, EventBus};
use execution::Executor;
use genesis::{GenesisBlock, GenesisLoader, GenesisState};
use metrics::NodeMetrics;
use networking::{NetworkConfig as NetCfg, NetworkNode};
use primitives::BlockHeight;
use security::{AntiSpam, ReplayProtection, SignatureCache};
use state::WorldState;
use storage::Storage;
use transaction::Mempool;

use crate::node::services::NodeServices;

/// Build and return all NodeServices from a NodeConfig.
///
/// This is called once during node startup.
pub fn build_services(config: &NodeConfig) -> Result<NodeServices, String> {
    // ── 1. Open storage ───────────────────────────────────────────────────────
    // Open storage backend based on config (RocksDB or InMemory).
    let mut storage = match config.storage.db_backend {
        DbBackend::InMemory => Storage::open_in_memory(),
        DbBackend::RocksDb  => {
            let data_dir = std::path::Path::new(&config.storage.data_dir);
            std::fs::create_dir_all(data_dir)
                .map_err(|e| format!("Cannot create data dir '{}': {}", data_dir.display(), e))?;
            Storage::open(data_dir)
                .map_err(|e| format!("Cannot open RocksDB at '{}': {}", data_dir.display(), e))?
        }
    };

    // ── 2. Load or build genesis ──────────────────────────────────────────────
    let genesis_cfg = GenesisLoader::default_config();

    let (genesis_block, initial_state) = match storage.get_block_at(BlockHeight::genesis())
        .map_err(|e| e.to_string())?
    {
        Some(existing_genesis) => {
            // Storage already has genesis — restore state from latest snapshot.
            let state = match storage.get_latest_snapshot().map_err(|e| e.to_string())? {
                Some(snap) => {
                    let mut s = WorldState::new();
                    s.restore_from_snapshot(snap);
                    s
                }
                None => GenesisState::build(&genesis_cfg).map_err(|e| e.to_string())?,
            };
            (existing_genesis, state)
        }
        None => {
            // First run — build genesis.
            let state  = GenesisState::build(&genesis_cfg).map_err(|e| e.to_string())?;
            let root   = state.state_root().clone();
            let block  = GenesisBlock::build(&genesis_cfg, root).map_err(|e| e.to_string())?;
            storage.save_block(&block).map_err(|e| e.to_string())?;
            (block, state)
        }
    };

    // ── 3. Build blockchain — restore all saved blocks ───────────────────────
    let initial_difficulty = config.consensus.initial_difficulty;
    let mut blockchain = Blockchain::new(genesis_block, initial_difficulty)
        .map_err(|e| e.to_string())?;

    // Replay all blocks from storage to restore chain state + executor state.
    let mut executor = execution::Executor::new(initial_state);
    let mut height = 1u64;
    loop {
        match storage.get_block_at(primitives::BlockHeight::new(height)).map_err(|e| e.to_string())? {
            Some(block) => {
                blockchain.add_block_unchecked(block.clone());
                let _ = executor.execute_block(&block); // restore state
                height += 1;
            }
            None => break,
        }
    }
    if height > 1 {
        eprintln!("[INFO] Restored blockchain — height=#{}", height - 1);
    }

    // ── 5. Init mempool ───────────────────────────────────────────────────────
    let mempool = Mempool::with_defaults();

    // ── 6. Init network ───────────────────────────────────────────────────────
    let net_config = networking::NetworkConfig {
        max_inbound:   config.network.max_inbound,
        max_outbound:  config.network.max_outbound,
        our_height:    blockchain.height().as_u64(),
        our_best_hash: blockchain.tip().hash(),
        listen_addr:   config.network.listen_addr.clone(),
    };
    let network = NetworkNode::new(net_config);

    // ── 7. Init event bus ─────────────────────────────────────────────────────
    let event_bus = EventBus::new();

    // ── 8. Init metrics and security ──────────────────────────────────────────
    let metrics           = NodeMetrics::new();
    let anti_spam         = AntiSpam::with_defaults();
    let replay_protection = ReplayProtection::with_defaults();
    let sig_cache         = SignatureCache::new(10_000);

    Ok(NodeServices {
        blockchain,
        executor,
        mempool,
        storage,
        network,
        event_bus,
        metrics,
        anti_spam,
        replay_protection,
        sig_cache,
    })
}

/// Graceful shutdown: save snapshot, flush storage, close all resources.
///
/// Order matters:
///   1. Stop accepting new work (emit NodeStopping)
///   2. Save final state snapshot (durability)
///   3. Flush storage write buffer (ensure data on disk)
///   4. Log final metrics
///   5. Drop services (RocksDB closes via Drop)
pub fn shutdown_services(services: &mut NodeServices) {
    eprintln!("[INFO] Graceful shutdown initiated...");

    // 1. Stop new work.
    services.event_bus.publish(ChainEvent::NodeStopping);

    // 2. Save final state snapshot.
    let snap = services.executor.snapshot();
    match services.storage.save_snapshot(&snap) {
        Ok(_)  => eprintln!("[INFO] State snapshot saved."),
        Err(e) => eprintln!("[WARN] Failed to save snapshot: {}", e),
    }

    // 3. Flush storage to disk (important for RocksDB write buffer).
    match services.storage.flush() {
        Ok(_)  => eprintln!("[INFO] Storage flushed."),
        Err(e) => eprintln!("[WARN] Storage flush failed: {}", e),
    }

    // 4. Log pending mempool transactions.
    let pending = services.mempool.len();
    if pending > 0 {
        eprintln!("[INFO] {} pending transactions in mempool at shutdown (not persisted).", pending);
    }

    // 5. Log final metrics.
    let m = &services.metrics;
    eprintln!(
        "[INFO] Node stopped — height={} peers={} mempool={} uptime={}s",
        m.chain_height,
        m.connected_peers,
        m.mempool_size,
        m.uptime_seconds(),
    );
    // Storage and network drop here — RocksDB flushes via Drop impl.
}
