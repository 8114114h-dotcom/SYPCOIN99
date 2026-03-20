// pow/difficulty.rs — Difficulty adjustment algorithm.
//
// Design (Bitcoin-style retargeting):
//
//   Every DIFFICULTY_ADJUSTMENT_INTERVAL blocks, the difficulty is adjusted
//   based on how long the previous interval actually took versus the target.
//
//   new_difficulty = current_difficulty × (target_time / actual_time)
//
//   Clamped to [current/4, current*4] to prevent wild swings.
//
//   Example:
//     target_time  = 2016 × 10s = 20160s
//     actual_time  = 10080s (twice as fast)
//     → new = current × (20160 / 10080) = current × 2  (harder)
//
//     actual_time  = 40320s (twice as slow)
//     → new = current × (20160 / 40320) = current / 2  (easier)
//
// Security:
//   • Integer arithmetic only — no floating point.
//   • The ×4/÷4 cap prevents a miner from manipulating timestamps to
//     artificially drop difficulty (time-warp attack mitigation).
//   • Minimum difficulty = 1 (never zero).

use primitives::BlockHeight;
use primitives::constants::{DIFFICULTY_ADJUSTMENT_INTERVAL, TARGET_BLOCK_TIME_MS};
use crate::pow::target::clamp_difficulty;

/// Returns `true` if difficulty should be recalculated at this height.
pub fn should_adjust(height: &BlockHeight) -> bool {
    height.as_u64() > 0 && height.as_u64() % DIFFICULTY_ADJUSTMENT_INTERVAL == 0
}

/// Calculate the new difficulty after an adjustment period.
///
/// # Arguments
/// - `current_difficulty` — difficulty at start of the adjustment period.
/// - `actual_time_ms`     — milliseconds elapsed over the adjustment interval.
///
/// # Returns
/// The new difficulty, clamped to [current/4, current*4].
pub fn adjust_difficulty(current_difficulty: u64, actual_time_ms: u64) -> u64 {
    let target_time_ms = DIFFICULTY_ADJUSTMENT_INTERVAL
        .saturating_mul(TARGET_BLOCK_TIME_MS);

    if actual_time_ms == 0 {
        // Avoid division by zero; clamp to maximum increase.
        return clamp_difficulty(current_difficulty,
            current_difficulty.saturating_mul(4));
    }

    // new = current × (target_time / actual_time)
    // Use u128 to avoid overflow during multiplication.
    let proposed = (current_difficulty as u128)
        .saturating_mul(target_time_ms as u128)
        / (actual_time_ms as u128);

    let proposed = proposed.min(u64::MAX as u128) as u64;

    clamp_difficulty(current_difficulty, proposed.max(1))
}

/// Calculate the expected total time for an adjustment interval.
pub fn target_interval_time_ms() -> u64 {
    DIFFICULTY_ADJUSTMENT_INTERVAL.saturating_mul(TARGET_BLOCK_TIME_MS)
}
