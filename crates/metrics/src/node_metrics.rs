// node_metrics.rs — Core node performance counters.
//
// NodeMetrics is a plain struct updated by the node runner after each
// significant event. It is read by the Prometheus exporter and the RPC
// layer (/metrics endpoint).
//
// Design:
//   • All counters are u64 — no atomics needed because NodeMetrics is
//     owned by the node runner and updated single-threadedly.
//   • hash_rate() is computed lazily from recent mining history rather
//     than maintained as a running average, avoiding drift.
//   • uptime() is derived from start_time — no stored counter.

use serde::{Deserialize, Serialize};
use primitives::Timestamp;

/// All node performance metrics in one struct.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeMetrics {
    // ── Chain ─────────────────────────────────────────────────────────────────
    /// Current canonical chain height.
    pub chain_height:       u64,
    /// Hash of the current best block (hex).
    pub best_block_hash:    String,
    /// Total transactions confirmed on-chain.
    pub total_transactions: u64,

    // ── Mining ────────────────────────────────────────────────────────────────
    /// Number of blocks this node has mined.
    pub blocks_mined:       u64,
    /// Nonces tried in the last mining session.
    pub last_nonces_tried:  u64,
    /// Milliseconds the last block took to mine.
    pub last_block_time_ms: u64,

    // ── Network ───────────────────────────────────────────────────────────────
    /// Currently connected peer count.
    pub connected_peers:    usize,
    /// Total bytes sent to peers.
    pub total_bytes_sent:   u64,
    /// Total bytes received from peers.
    pub total_bytes_recv:   u64,
    /// Total blocks received from the network.
    pub total_blocks_recv:  u64,
    /// Total transactions received from the network.
    pub total_txs_recv:     u64,

    // ── Mempool ───────────────────────────────────────────────────────────────
    /// Current number of pending transactions in the mempool.
    pub mempool_size:       usize,

    // ── System ────────────────────────────────────────────────────────────────
    /// Wall-clock time when the node started.
    pub start_time:         Timestamp,
}

impl NodeMetrics {
    /// Create a fresh metrics instance with the node's start time.
    pub fn new() -> Self {
        NodeMetrics {
            chain_height:       0,
            best_block_hash:    "0".repeat(64),
            total_transactions: 0,
            blocks_mined:       0,
            last_nonces_tried:  0,
            last_block_time_ms: 0,
            connected_peers:    0,
            total_bytes_sent:   0,
            total_bytes_recv:   0,
            total_blocks_recv:  0,
            total_txs_recv:     0,
            mempool_size:       0,
            start_time:         Timestamp::now(),
        }
    }

    // ── Update methods ────────────────────────────────────────────────────────

    /// Update chain state after a new block is applied.
    pub fn update_chain(
        &mut self,
        height:    u64,
        best_hash: String,
        tx_count:  u64,
    ) {
        self.chain_height       = height;
        self.best_block_hash    = best_hash;
        self.total_transactions = self.total_transactions.saturating_add(tx_count);
    }

    /// Update mining stats after a block is mined.
    pub fn update_mining(&mut self, nonces_tried: u64, elapsed_ms: u64) {
        self.blocks_mined       = self.blocks_mined.saturating_add(1);
        self.last_nonces_tried  = nonces_tried;
        self.last_block_time_ms = elapsed_ms;
    }

    /// Update network counters.
    pub fn update_network(
        &mut self,
        peers:       usize,
        bytes_sent:  u64,
        bytes_recv:  u64,
        blocks_recv: u64,
        txs_recv:    u64,
    ) {
        self.connected_peers   = peers;
        self.total_bytes_sent  = self.total_bytes_sent.saturating_add(bytes_sent);
        self.total_bytes_recv  = self.total_bytes_recv.saturating_add(bytes_recv);
        self.total_blocks_recv = self.total_blocks_recv.saturating_add(blocks_recv);
        self.total_txs_recv    = self.total_txs_recv.saturating_add(txs_recv);
    }

    /// Update mempool stats.
    pub fn update_mempool(&mut self, size: usize) {
        self.mempool_size = size;
    }

    // ── Derived metrics ───────────────────────────────────────────────────────

    /// Approximate hash rate from the last mining session (hashes/sec).
    ///
    /// Returns 0.0 if no mining has occurred or the last block took 0ms.
    pub fn hash_rate(&self) -> f64 {
        if self.last_block_time_ms == 0 || self.last_nonces_tried == 0 {
            return 0.0;
        }
        (self.last_nonces_tried as f64) / (self.last_block_time_ms as f64 / 1000.0)
    }

    /// Seconds since the node started.
    pub fn uptime_seconds(&self) -> u64 {
        Timestamp::now()
            .millis_since(&self.start_time)
            .unwrap_or(0)
            / 1000
    }
}

impl Default for NodeMetrics {
    fn default() -> Self { Self::new() }
}
