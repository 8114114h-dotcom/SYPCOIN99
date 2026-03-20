// events.rs — All events emitted by the node subsystems.
//
// Design:
//   • ChainEvent is Clone so the bus can fan-out to multiple subscribers.
//   • #[non_exhaustive] allows adding new variants without breaking
//     existing match arms in downstream code.
//   • Each variant carries only the minimal data needed by subscribers.
//     Large types (Block, Transaction) are referenced by hash — subscribers
//     that need the full data query storage directly.

use serde::{Deserialize, Serialize};

use crypto::HashDigest;
use primitives::{BlockHeight, Timestamp};

/// All events that can flow through the event bus.
#[non_exhaustive]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ChainEvent {
    // ── Chain ─────────────────────────────────────────────────────────────────
    /// A new block was added to the canonical chain.
    BlockAdded {
        block_hash: HashDigest,
        height:     BlockHeight,
        tx_count:   u32,
        timestamp:  Timestamp,
    },

    /// A block was removed from the canonical chain (during reorg).
    BlockReverted {
        block_hash: HashDigest,
        height:     BlockHeight,
    },

    /// The canonical chain was reorganised.
    ChainReorganized {
        old_tip:    HashDigest,
        new_tip:    HashDigest,
        depth:      u64,       // how many blocks were reverted
    },

    // ── Transactions ──────────────────────────────────────────────────────────
    /// A new transaction was received and added to the mempool.
    NewTransaction {
        tx_id:    HashDigest,
        from:     String,      // checksum hex
        to:       String,
        amount:   u64,         // micro-tokens
    },

    /// A transaction was included in a confirmed block.
    TransactionConfirmed {
        tx_id:        HashDigest,
        block_height: BlockHeight,
        block_hash:   HashDigest,
    },

    /// A transaction was evicted from the mempool (expired or replaced).
    TransactionEvicted {
        tx_id:  HashDigest,
        reason: String,
    },

    // ── Mining ────────────────────────────────────────────────────────────────
    /// The miner started working on a new block template.
    MiningStarted {
        height:     BlockHeight,
        difficulty: u64,
    },

    /// The miner was stopped (e.g. new block received from network).
    MiningStopped,

    /// The miner found a valid block.
    BlockMined {
        block_hash: HashDigest,
        height:     BlockHeight,
        nonce:      u64,
        elapsed_ms: u64,
    },

    // ── Networking ────────────────────────────────────────────────────────────
    /// A new peer connected successfully (handshake complete).
    PeerConnected {
        peer_addr: String,
        peer_height: u64,
    },

    /// A peer disconnected or was banned.
    PeerDisconnected {
        peer_addr: String,
        reason:    String,
    },

    // ── Sync ─────────────────────────────────────────────────────────────────
    /// Chain synchronisation with a peer started.
    SyncStarted {
        from_height: u64,
        to_height:   u64,
        peer_addr:   String,
    },

    /// Chain synchronisation completed.
    SyncCompleted {
        height: BlockHeight,
    },

    // ── Node lifecycle ────────────────────────────────────────────────────────
    /// The node finished startup and is ready.
    NodeStarted {
        height:   BlockHeight,
        tip_hash: HashDigest,
    },

    /// The node is shutting down gracefully.
    NodeStopping,
}

impl ChainEvent {
    /// Human-readable event type name for logging.
    pub fn type_name(&self) -> &'static str {
        match self {
            ChainEvent::BlockAdded { .. }           => "BlockAdded",
            ChainEvent::BlockReverted { .. }        => "BlockReverted",
            ChainEvent::ChainReorganized { .. }     => "ChainReorganized",
            ChainEvent::NewTransaction { .. }       => "NewTransaction",
            ChainEvent::TransactionConfirmed { .. } => "TransactionConfirmed",
            ChainEvent::TransactionEvicted { .. }   => "TransactionEvicted",
            ChainEvent::MiningStarted { .. }        => "MiningStarted",
            ChainEvent::MiningStopped               => "MiningStopped",
            ChainEvent::BlockMined { .. }           => "BlockMined",
            ChainEvent::PeerConnected { .. }        => "PeerConnected",
            ChainEvent::PeerDisconnected { .. }     => "PeerDisconnected",
            ChainEvent::SyncStarted { .. }          => "SyncStarted",
            ChainEvent::SyncCompleted { .. }        => "SyncCompleted",
            ChainEvent::NodeStarted { .. }          => "NodeStarted",
            ChainEvent::NodeStopping                => "NodeStopping",
        }
    }
}
