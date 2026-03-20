// sync/fast_sync.rs — Snapshot-based fast sync.
//
// Instead of replaying all blocks from genesis, a new node can:
//   1. Download a recent state snapshot from a trusted peer.
//   2. Verify the snapshot's state_root against the block header.
//   3. Continue with normal sync from the snapshot height.
//
// This reduces initial sync from O(all blocks) to O(recent blocks).

use primitives::BlockHeight;

/// State of a fast sync session.
#[derive(Debug, PartialEq, Eq)]
pub enum FastSyncState {
    /// Waiting to start.
    Idle,
    /// Requested snapshot at this height from a peer.
    AwaitingSnapshot { height: BlockHeight },
    /// Snapshot received, verifying state root.
    Verifying { height: BlockHeight },
    /// Snapshot applied, resuming normal sync from here.
    Complete { resumed_at: BlockHeight },
    /// Fast sync failed, falling back to full sync.
    Failed { reason: String },
}

pub struct FastSync {
    pub state: FastSyncState,
}

impl FastSync {
    pub fn new() -> Self {
        FastSync { state: FastSyncState::Idle }
    }

    pub fn start(&mut self, height: BlockHeight) {
        self.state = FastSyncState::AwaitingSnapshot { height };
    }

    pub fn on_snapshot_received(&mut self, height: BlockHeight) {
        self.state = FastSyncState::Verifying { height };
    }

    pub fn on_verified(&mut self, height: BlockHeight) {
        self.state = FastSyncState::Complete { resumed_at: height };
    }

    pub fn on_failed(&mut self, reason: String) {
        self.state = FastSyncState::Failed { reason };
    }

    pub fn is_complete(&self) -> bool {
        matches!(self.state, FastSyncState::Complete { .. })
    }
}

impl Default for FastSync {
    fn default() -> Self { Self::new() }
}
