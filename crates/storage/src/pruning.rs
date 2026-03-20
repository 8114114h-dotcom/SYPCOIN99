// pruning.rs — Old snapshot pruning policy.
//
// Snapshots accumulate over time. We keep the N most recent snapshots
// and delete older ones to bound disk usage.
//
// Pruning is NOT consensus-critical: each node may prune independently.
// The only constraint is that we keep enough snapshots to handle reorgs
// up to MAX_REORG_DEPTH blocks deep.

use crate::error::StorageError;
use crate::repositories::state_repo::StateRepository;

/// Keep at least this many snapshots regardless of the prune request.
/// Ensures we can always roll back MAX_REORG_DEPTH (100) blocks.
pub const MIN_SNAPSHOTS_TO_KEEP: usize = 10;

/// Prune old snapshots, keeping the `keep` most recent.
///
/// Returns the number of snapshots deleted.
pub fn prune_old_snapshots(
    repo: &StateRepository,
    keep: usize,
) -> Result<usize, StorageError> {
    let keep = keep.max(MIN_SNAPSHOTS_TO_KEEP);
    let heights = repo.list_snapshot_heights()?;

    if heights.len() <= keep {
        return Ok(0);
    }

    // Heights are in ascending order; delete the oldest ones.
    let to_delete = &heights[..heights.len() - keep];
    for &height in to_delete {
        repo.delete_snapshot(height)?;
    }
    Ok(to_delete.len())
}
