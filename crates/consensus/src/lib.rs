// lib.rs — Public API surface for the `consensus` crate.
//
//   use consensus::{Blockchain, Miner, MineResult};
//   use consensus::{ChainRules, DifficultyAdjuster};
//   use consensus::{evaluate_reorg, ReorgDecision};
//   use consensus::ConsensusError;

mod error;
mod fork_choice;
mod chain_rules;

mod pow {
    pub(crate) mod difficulty;
    pub(crate) mod miner;
    pub(crate) mod target;
}

mod chain {
    pub(crate) mod blockchain;
    pub(crate) mod fork;
    pub(crate) mod reorg;
}

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use error::ConsensusError;
pub use chain::blockchain::Blockchain;
pub use chain::reorg::{evaluate_reorg, ReorgDecision};
pub use chain::fork::{find_common_ancestor, ForkInfo};
pub use chain_rules::ChainRules;
pub use pow::miner::{Miner, MineResult};
pub use pow::difficulty::{adjust_difficulty, should_adjust, target_interval_time_ms};
pub use pow::target::clamp_difficulty;
pub use fork_choice::{
    cumulative_work, is_better_chain,
    validate_reorg_depth, MAX_REORG_DEPTH,
};

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use block::{Block, BlockBuilder, difficulty_to_target, meets_target};
    use crypto::{Address, KeyPair, sha256};
    use primitives::{BlockHeight, Timestamp};
    use primitives::constants::{DIFFICULTY_ADJUSTMENT_INTERVAL, TARGET_BLOCK_TIME_MS};

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn make_address() -> Address {
        Address::from_public_key(KeyPair::generate().unwrap().public_key())
    }

    fn zero_hash() -> crypto::HashDigest {
        sha256(b"zero")
    }

    /// Mine a block with difficulty=1 (trivially easy — any hash passes).
    fn mine_block(height: u64, parent_hash: crypto::HashDigest) -> Block {
        let template = BlockBuilder::new()
            .height(BlockHeight::new(height))
            .parent_hash(parent_hash)
            .state_root(zero_hash())
            .miner(make_address())
            .difficulty(1)
            .timestamp(Timestamp::from_millis(
                1_700_000_000_000 + height * 10_000
            ))
            .build()
            .unwrap();

        let miner = Miner::new(make_address());
        miner.mine(template, || false).unwrap().block
    }

    fn make_genesis() -> Block {
        mine_block(0, zero_hash())
    }

    // ── Difficulty adjustment ─────────────────────────────────────────────────

    #[test]
    fn test_should_adjust_at_interval() {
        assert!(!should_adjust(&BlockHeight::new(0)));
        assert!(!should_adjust(&BlockHeight::new(1)));
        assert!(should_adjust(&BlockHeight::new(DIFFICULTY_ADJUSTMENT_INTERVAL)));
        assert!(should_adjust(&BlockHeight::new(DIFFICULTY_ADJUSTMENT_INTERVAL * 2)));
    }

    #[test]
    fn test_adjust_difficulty_too_fast() {
        // Blocks came in twice as fast → difficulty doubles.
        let current = 1000u64;
        let target_ms = DIFFICULTY_ADJUSTMENT_INTERVAL * TARGET_BLOCK_TIME_MS;
        let actual_ms = target_ms / 2; // twice as fast
        let new_diff  = adjust_difficulty(current, actual_ms);
        assert!(new_diff > current, "difficulty should increase");
        assert!(new_diff <= current * 4, "capped at 4×");
    }

    #[test]
    fn test_adjust_difficulty_too_slow() {
        // Blocks came in twice as slow → difficulty halves.
        let current = 1000u64;
        let target_ms = DIFFICULTY_ADJUSTMENT_INTERVAL * TARGET_BLOCK_TIME_MS;
        let actual_ms = target_ms * 2; // twice as slow
        let new_diff  = adjust_difficulty(current, actual_ms);
        assert!(new_diff < current, "difficulty should decrease");
        assert!(new_diff >= current / 4, "floored at ÷4");
    }

    #[test]
    fn test_adjust_difficulty_on_target() {
        let current   = 1000u64;
        let target_ms = DIFFICULTY_ADJUSTMENT_INTERVAL * TARGET_BLOCK_TIME_MS;
        let new_diff  = adjust_difficulty(current, target_ms);
        assert_eq!(new_diff, current, "no change when on target");
    }

    #[test]
    fn test_difficulty_never_below_one() {
        let new_diff = adjust_difficulty(1, u64::MAX);
        assert!(new_diff >= 1);
    }

    // ── PoW target ────────────────────────────────────────────────────────────

    #[test]
    fn test_difficulty_1_trivially_easy() {
        // With difficulty=1, almost every hash should pass.
        let target = difficulty_to_target(1);
        let hash   = sha256(b"any input");
        // At difficulty=1, the target is very high — most hashes pass.
        // We just verify the comparison doesn't panic.
        let _ = meets_target(&hash, &target);
    }

    #[test]
    fn test_clamp_difficulty_upper_bound() {
        let clamped = pow::target::clamp_difficulty(100, 9999);
        assert!(clamped <= 400, "must be ≤ 4× current");
    }

    #[test]
    fn test_clamp_difficulty_lower_bound() {
        let clamped = pow::target::clamp_difficulty(100, 1);
        assert!(clamped >= 25, "must be ≥ current/4");
    }

    // ── Miner ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_mine_block_difficulty_1() {
        let template = BlockBuilder::new()
            .height(BlockHeight::new(1))
            .parent_hash(zero_hash())
            .state_root(zero_hash())
            .miner(make_address())
            .difficulty(1)
            .timestamp(Timestamp::now())
            .build()
            .unwrap();

        let miner  = Miner::new(make_address());
        let result = miner.mine(template, || false);
        assert!(result.is_ok(), "difficulty=1 must always succeed");
        let mr = result.unwrap();
        assert!(mr.nonces_tried >= 1);
    }

    #[test]
    fn test_mine_cancel_immediately() {
        let template = BlockBuilder::new()
            .height(BlockHeight::new(1))
            .parent_hash(zero_hash())
            .state_root(zero_hash())
            .miner(make_address())
            .difficulty(u64::MAX) // impossible
            .timestamp(Timestamp::now())
            .build()
            .unwrap();

        let miner  = Miner::new(make_address());
        // Cancel immediately on first check.
        let result = miner.mine(template, || true);
        assert!(matches!(result, Err(ConsensusError::MiningFailed)));
    }

    // ── Blockchain ────────────────────────────────────────────────────────────

    #[test]
    fn test_blockchain_genesis() {
        let genesis    = make_genesis();
        let blockchain = Blockchain::new(genesis, 1).unwrap();
        assert_eq!(blockchain.height(), BlockHeight::new(0));
        assert_eq!(blockchain.len(), 1);
    }

    #[test]
    fn test_blockchain_add_block() {
        let genesis    = make_genesis();
        let genesis_hash = genesis.hash();
        let mut chain  = Blockchain::new(genesis, 1).unwrap();

        let block1 = mine_block(1, genesis_hash);
        chain.add_block(block1).unwrap();
        assert_eq!(chain.height(), BlockHeight::new(1));
        assert_eq!(chain.len(), 2);
    }

    #[test]
    fn test_blockchain_duplicate_rejected() {
        let genesis      = make_genesis();
        let genesis_hash = genesis.hash();
        let mut chain    = Blockchain::new(genesis, 1).unwrap();
        let block1       = mine_block(1, genesis_hash);
        chain.add_block(block1.clone()).unwrap();
        let result = chain.add_block(block1);
        assert!(matches!(result, Err(ConsensusError::BlockAlreadyKnown)));
    }

    #[test]
    fn test_blockchain_lookup_by_hash() {
        let genesis      = make_genesis();
        let genesis_hash = genesis.hash();
        let chain        = Blockchain::new(genesis, 1).unwrap();
        assert!(chain.contains(&genesis_hash));
        assert!(chain.get_block(&genesis_hash).is_some());
    }

    #[test]
    fn test_blockchain_lookup_by_height() {
        let genesis = make_genesis();
        let chain   = Blockchain::new(genesis, 1).unwrap();
        assert!(chain.get_block_at(BlockHeight::new(0)).is_some());
        assert!(chain.get_block_at(BlockHeight::new(1)).is_none());
    }

    // ── Fork choice ───────────────────────────────────────────────────────────

    #[test]
    fn test_is_better_chain_higher_work_wins() {
        let h1 = BlockBuilder::new()
            .height(BlockHeight::new(1))
            .parent_hash(zero_hash())
            .state_root(zero_hash())
            .miner(make_address())
            .difficulty(1)
            .timestamp(Timestamp::now())
            .build()
            .unwrap();

        let h2 = BlockBuilder::new()
            .height(BlockHeight::new(1))
            .parent_hash(zero_hash())
            .state_root(zero_hash())
            .miner(make_address())
            .difficulty(1)
            .timestamp(Timestamp::now())
            .build()
            .unwrap();

        // candidate has 2000 work, current has 1000 → candidate wins.
        let candidate_better = is_better_chain(
            2000, h1.header(),
            1000, h2.header(),
        );
        assert!(candidate_better);

        let current_better = is_better_chain(
            1000, h1.header(),
            2000, h2.header(),
        );
        assert!(!current_better);
    }

    #[test]
    fn test_reorg_depth_limit() {
        assert!(validate_reorg_depth(0, MAX_REORG_DEPTH).is_ok());
        assert!(validate_reorg_depth(0, MAX_REORG_DEPTH + 1).is_err());
    }

    // ── Reorg decision ────────────────────────────────────────────────────────

    #[test]
    fn test_evaluate_reorg_better_candidate() {
        let block = BlockBuilder::new()
            .height(BlockHeight::new(1))
            .parent_hash(zero_hash())
            .state_root(zero_hash())
            .miner(make_address())
            .difficulty(100)
            .timestamp(Timestamp::now())
            .build()
            .unwrap();

        let low = BlockBuilder::new()
            .height(BlockHeight::new(1))
            .parent_hash(zero_hash())
            .state_root(zero_hash())
            .miner(make_address())
            .difficulty(10)
            .timestamp(Timestamp::now())
            .build()
            .unwrap();

        // candidate chain has difficulty=100 > current difficulty=10
        let decision = evaluate_reorg(
            &[low.header()],
            &[block.header()],
        ).unwrap();
        assert_eq!(decision, ReorgDecision::Reorg);
    }

    #[test]
    fn test_evaluate_reorg_keep_current() {
        let strong = BlockBuilder::new()
            .height(BlockHeight::new(1))
            .parent_hash(zero_hash())
            .state_root(zero_hash())
            .miner(make_address())
            .difficulty(1000)
            .timestamp(Timestamp::now())
            .build()
            .unwrap();

        let weak = BlockBuilder::new()
            .height(BlockHeight::new(1))
            .parent_hash(zero_hash())
            .state_root(zero_hash())
            .miner(make_address())
            .difficulty(10)
            .timestamp(Timestamp::now())
            .build()
            .unwrap();

        let decision = evaluate_reorg(
            &[strong.header()],
            &[weak.header()],
        ).unwrap();
        assert_eq!(decision, ReorgDecision::KeepCurrent);
    }

    // ── ChainRules ────────────────────────────────────────────────────────────

    #[test]
    fn test_chain_rules_next_difficulty_no_adjust() {
        // At height=1 (not an adjustment boundary), difficulty stays same.
        let diff = ChainRules::next_difficulty(
            1000,
            &BlockHeight::new(1),
            0,
            10_000,
        );
        assert_eq!(diff, 1000);
    }

    #[test]
    fn test_chain_rules_min_max_difficulty() {
        assert_eq!(ChainRules::min_difficulty(100), 25);
        assert_eq!(ChainRules::max_difficulty(100), 400);
        assert_eq!(ChainRules::min_difficulty(1), 1); // never below 1
    }
}

