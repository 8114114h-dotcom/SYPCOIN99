// chain/reorg.rs — Chain reorganisation decision with 51% attack protection.
//
// Security decisions:
//
//   MAX_REORG_DEPTH (100 blocks) — hard limit prevents long-range attacks.
//   Even a majority miner cannot rewrite more than 100 blocks of history.
//   This is the primary defence against 51% attacks on confirmed transactions.
//
//   CUMULATIVE WORK — we select the chain with more total PoW, not the longer
//   chain. This prevents "selfish mining" attacks where an attacker withholds
//   blocks.
//
//   TIMESTAMPS — blocks too far in the future are rejected at the validation
//   layer before reorg evaluation, preventing time-warp attacks.

use block::BlockHeader;

use crate::error::ConsensusError;
use crate::fork_choice::{cumulative_work, is_better_chain, validate_reorg_depth, MAX_REORG_DEPTH};

/// Outcome of a reorg evaluation.
#[derive(Debug, PartialEq, Eq)]
pub enum ReorgDecision {
    /// The new chain is better — perform the reorg.
    Reorg,
    /// The current chain is better — ignore the competing fork.
    KeepCurrent,
}

/// Decide whether to reorg given two competing chain tips.
///
/// Returns `Err` if the reorg depth exceeds `MAX_REORG_DEPTH` (anti-51% guard).
///
/// # Security
/// A reorg deeper than `MAX_REORG_DEPTH` is rejected regardless of cumulative
/// work. This means an attacker controlling >50% hashrate still cannot rewrite
/// more than 100 confirmed blocks. Applications should treat transactions
/// confirmed at depth > 100 as irreversible.
pub fn evaluate_reorg(
    current_height:    u64,
    fork_point_height: u64,
    current_headers:   &[&BlockHeader],
    candidate_headers: &[&BlockHeader],
) -> Result<ReorgDecision, ConsensusError> {
    if current_headers.is_empty() || candidate_headers.is_empty() {
        return Err(ConsensusError::ChainNotFound);
    }

    // ── Anti-51% / long-range attack guard ───────────────────────────────────
    // Reject reorgs deeper than MAX_REORG_DEPTH regardless of work.
    validate_reorg_depth(fork_point_height, current_height)?;

    let reorg_depth = current_height.saturating_sub(fork_point_height);
    if reorg_depth > MAX_REORG_DEPTH {
        return Err(ConsensusError::ReorgTooDeep {
            depth: reorg_depth,
            max:   MAX_REORG_DEPTH,
        });
    }

    // ── Heaviest chain rule ───────────────────────────────────────────────────
    let current_work   = cumulative_work(current_headers);
    let candidate_work = cumulative_work(candidate_headers);

    let current_tip   = current_headers.last().unwrap();
    let candidate_tip = candidate_headers.last().unwrap();

    if is_better_chain(candidate_work, candidate_tip, current_work, current_tip) {
        Ok(ReorgDecision::Reorg)
    } else {
        Ok(ReorgDecision::KeepCurrent)
    }
}
