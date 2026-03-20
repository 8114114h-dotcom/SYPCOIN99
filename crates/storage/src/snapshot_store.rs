// snapshot_store.rs — High-level snapshot management.
//
// Wraps StateRepository and pruning logic into a single convenient API
// used by the node layer.

use primitives::BlockHeight;
use state::StateSnapshot;

use crate::error::StorageError;
use crate::pruning::prune_old_snapshots;
use crate::repositories::state_repo::StateRepository;

/// Manages state snapshots: save, load, list, prune.
pub struct SnapshotStore {
    repo: StateRepository,
    /// Number of snapshots to keep after pruning.
    keep: usize,
}

impl SnapshotStore {
    pub fn new(repo: StateRepository, keep: usize) -> Self {
        SnapshotStore { repo, keep }
    }

    /// Save a snapshot and auto-prune if over the keep limit.
    pub fn save_and_prune(&self, snap: &StateSnapshot) -> Result<usize, StorageError> {
        self.repo.save_snapshot(snap)?;
        prune_old_snapshots(&self.repo, self.keep)
    }

    pub fn get(&self, height: BlockHeight) -> Result<Option<StateSnapshot>, StorageError> {
        self.repo.get_snapshot(height)
    }

    pub fn get_latest(&self) -> Result<Option<StateSnapshot>, StorageError> {
        self.repo.get_latest_snapshot()
    }

    pub fn list_heights(&self) -> Result<Vec<BlockHeight>, StorageError> {
        self.repo.list_snapshot_heights()
    }
}
