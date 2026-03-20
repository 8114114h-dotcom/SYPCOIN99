// fork_choice.rs — Fork choice rule: longest chain (most cumulative work).
//
// Design:
//   We use the "heaviest chain" rule: the chain with the most cumulative
//   proof-of-work wins. For equal cumulative work, the chain seen first wins
//   (ties are broken by block hash lexicographic order for determinism).
//
//   cumulative_work ≈ sum(difficulty) for all blocks in the chain.
//
//   This matches Bitcoin's chain selection rule and is Sybil-resistant:
//   an attacker must redo all PoW to rewrite history.
//
// MAX_REORG_DEPTH:
//   We refuse to reorg deeper than MAX_REORG_DEPTH blocks. This protects
//   against long-range attacks where an attacker mines a secret chain offline
//   and then broadcasts it to cause a deep reorganisation.

use block::BlockHeader;
use crate::error::ConsensusError;

/// Maximum number of blocks that can be reorganised in a single reorg.
/// CONSENSUS CRITICAL — nodes that disagree on this value will diverge.
// Use primitives::constants::MAX_REORG_DEPTH as single source of truth.
pub use primitives::constants::MAX_REORG_DEPTH;

/// Cumulative work for a chain, approximated as the sum of difficulties.
///
/// Using u128 to avoid overflow: u64::MAX × 1_000_000 blocks < u128::MAX.
pub fn cumulative_work(headers: &[&BlockHeader]) -> u128 {
    headers.iter().map(|h| h.difficulty() as u128).sum()
}

/// Returns `true` if `candidate` represents a better chain tip than `current`.
///
/// "Better" = higher cumulative work. Tie-broken by block hash (lower = better,
/// matching Bitcoin's behaviour for deterministic resolution).
pub fn is_better_chain(
    candidate_work: u128,
    candidate_tip:  &BlockHeader,
    current_work:   u128,
    current_tip:    &BlockHeader,
) -> bool {
    if candidate_work != current_work {
        return candidate_work > current_work;
    }
    // Tie-break: lexicographically smaller hash wins.
    candidate_tip.hash().as_bytes() < current_tip.hash().as_bytes()
}

/// Validate that a potential reorg does not exceed MAX_REORG_DEPTH.
pub fn validate_reorg_depth(
    common_ancestor_height: u64,
    current_tip_height:     u64,
) -> Result<(), ConsensusError> {
    let depth = current_tip_height.saturating_sub(common_ancestor_height);
    if depth > MAX_REORG_DEPTH {
        return Err(ConsensusError::ForkTooDeep { depth });
    }
    Ok(())
}
