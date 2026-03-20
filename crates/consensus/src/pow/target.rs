// pow/target.rs — Re-exports and consensus-level target utilities.
//
// The core difficulty→target conversion lives in block::block_hash to avoid
// circular dependencies. This module provides consensus-level helpers that
// use that conversion.



/// Maximum allowed difficulty multiplier per adjustment period.
/// Bitcoin uses 4× — we follow the same convention.
pub const MAX_DIFFICULTY_FACTOR: u64 = 4;

/// Minimum difficulty (never go below 1).
pub const MIN_DIFFICULTY: u64 = 1;

/// Clamp a new difficulty value within the allowed adjustment range.
///
/// new_difficulty is clamped to [current/4, current*4] to prevent
/// sudden massive difficulty swings that could be exploited.
pub fn clamp_difficulty(current: u64, proposed: u64) -> u64 {
    let min = (current / MAX_DIFFICULTY_FACTOR).max(MIN_DIFFICULTY);
    let max = current.saturating_mul(MAX_DIFFICULTY_FACTOR);
    proposed.clamp(min, max)
}
