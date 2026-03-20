// chain/fork.rs — Fork detection and chain reorganisation helpers.
//
// A fork occurs when two blocks at the same height both have valid PoW.
// The node keeps the chain with the most cumulative work (heaviest chain rule).
//
// Reorg procedure:
//   1. Find the common ancestor of the current chain and the fork.
//   2. Validate reorg depth ≤ MAX_REORG_DEPTH.
//   3. Roll back state to the common ancestor (state layer handles this).
//   4. Apply blocks on the new fork.
//
// This module handles steps 1 and 2 only. Steps 3-4 are orchestrated by
// the node layer using WorldState::restore_from_snapshot() + apply_block().

use std::collections::HashMap;
use crypto::HashDigest;
use block::BlockHeader;

use crate::error::ConsensusError;
use crate::fork_choice::validate_reorg_depth;

/// Describes a detected fork.
#[derive(Debug)]
pub struct ForkInfo {
    /// Height of the common ancestor block.
    pub common_ancestor_height: u64,
    /// Hash of the common ancestor block.
    pub common_ancestor_hash:   HashDigest,
    /// How many blocks must be rolled back on the current chain.
    pub rollback_depth:         u64,
}

/// Find the common ancestor between two chains.
///
/// # Arguments
/// - `main_headers`   — headers of the current main chain, indexed by hash.
/// - `fork_headers`   — headers of the competing fork, from tip to genesis.
///
/// # Returns
/// `ForkInfo` if a common ancestor is found within MAX_REORG_DEPTH.
pub fn find_common_ancestor(
    main_headers: &HashMap<String, BlockHeader>,
    fork_headers: &[BlockHeader],
    current_tip_height: u64,
) -> Result<ForkInfo, ConsensusError> {
    for fork_header in fork_headers {
        let hash_hex = hex::encode(fork_header.hash().as_bytes());
        if main_headers.contains_key(&hash_hex) {
            let common_height  = fork_header.height().as_u64();
            let rollback_depth = current_tip_height.saturating_sub(common_height);

            validate_reorg_depth(common_height, current_tip_height)?;

            return Ok(ForkInfo {
                common_ancestor_height: common_height,
                common_ancestor_hash:   fork_header.hash(),
                rollback_depth,
            });
        }
    }
    Err(ConsensusError::OrphanBlock)
}
