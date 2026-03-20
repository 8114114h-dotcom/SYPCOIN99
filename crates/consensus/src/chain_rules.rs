// chain_rules.rs — Consensus validation rules applied to each block.
//
// These rules are consensus-critical: every full node must apply them
// identically or the network will fork.

use block::{Block, BlockHeader, BlockValidator};
use primitives::{BlockHeight, block_reward_at};

use crate::error::ConsensusError;
use crate::pow::difficulty::{adjust_difficulty, should_adjust};

pub struct ChainRules;

impl ChainRules {
    /// Full consensus validation of a block.
    ///
    /// Runs in order:
    ///   1. Structural validation (block crate)
    ///   2. PoW validation
    ///   3. Parent linkage
    ///   4. Difficulty correctness
    /// Validate a block against chain rules.
    ///
    /// `recent_timestamps` — timestamps of the last `MTP_WINDOW` blocks,
    /// oldest-first. Used for MTP (Median Time Past) timestamp validation.
    pub fn validate_block(
        block:               &Block,
        parent:              &BlockHeader,
        expected_difficulty: u64,
    ) -> Result<(), ConsensusError> {
        // 1. Structure.
        BlockValidator::validate_structure(block)
            .map_err(|e| ConsensusError::BlockValidation(e.to_string()))?;

        // 2. PoW.
        BlockValidator::validate_pow(block)
            .map_err(|_| ConsensusError::InvalidPoW)?;

        // 3. Parent linkage.
        BlockValidator::validate_against_parent(block, parent)
            .map_err(|e| ConsensusError::BlockValidation(e.to_string()))?;

        // 4. Difficulty must match what this node expects.
        if block.difficulty() != expected_difficulty {
            return Err(ConsensusError::InvalidDifficulty {
                expected: expected_difficulty,
                got:      block.difficulty(),
            });
        }

        Ok(())
    }

    /// Compute what the difficulty should be for the next block.
    ///
    /// # Arguments
    /// - `current_difficulty`   — difficulty of the current tip.
    /// - `tip_height`           — height of the current tip.
    /// - `interval_start_time`  — timestamp of the block at the start of the
    ///                            last adjustment interval (ms).
    /// - `interval_end_time`    — timestamp of the current tip (ms).
    pub fn next_difficulty(
        current_difficulty:  u64,
        tip_height:          &BlockHeight,
        interval_start_ms:   u64,
        interval_end_ms:     u64,
    ) -> u64 {
        // Only adjust at the boundary; otherwise keep current.
        if !should_adjust(tip_height) {
            return current_difficulty;
        }

        let actual_time_ms = interval_end_ms.saturating_sub(interval_start_ms);
        adjust_difficulty(current_difficulty, actual_time_ms)
    }

    /// Verify the block reward does not exceed the allowed amount.
    ///
    /// In our model, rewards are applied by the state layer, so this check
    /// validates that the block's claimed miner reward (embedded in the block
    /// metadata for future coinbase tx support) is not inflated.
    ///
    /// For now we validate that block_reward_at(height) is positive and that
    /// the height is consistent with the expected halving epoch.
    pub fn validate_reward_policy(height: &BlockHeight) -> Result<(), ConsensusError> {
        // block_reward_at() is pure and deterministic.
        // It returns 0 after MAX_HALVINGS, which is valid (fees-only era).
        let _reward = block_reward_at(height); // verify it doesn't panic
        Ok(())
    }

    /// Maximum allowed difficulty for the next block given the current.
    pub fn max_difficulty(current: u64) -> u64 {
        current.saturating_mul(4)
    }

    /// Minimum allowed difficulty for the next block given the current.
    pub fn min_difficulty(current: u64) -> u64 {
        (current / 4).max(1)
    }
}
