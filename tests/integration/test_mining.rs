// integration/test_mining.rs
// End-to-end mining: mine real blocks and verify chain integrity.

use block::BlockBuilder;
use consensus::{Blockchain, Miner};
use crypto::{Address, KeyPair, sha256};
use execution::Executor;
use genesis::{GenesisBlock, GenesisLoader, GenesisState};
use primitives::{BlockHeight, Timestamp, block_reward_at};
use state::WorldState;

fn make_address() -> Address {
    Address::from_public_key(KeyPair::generate().unwrap().public_key())
}

#[test]
fn test_mine_10_blocks_chain_integrity() {
    let miner_addr = make_address();
    let cfg        = GenesisLoader::default_config();
    let state      = GenesisState::build(&cfg).unwrap();
    let root       = state.state_root().clone();
    let genesis    = GenesisBlock::build(&cfg, root).unwrap();

    let mut chain    = Blockchain::new(genesis.clone(), 1).unwrap();
    let mut executor = Executor::new(state);
    let miner        = Miner::new(miner_addr.clone());
    let mut parent   = genesis.hash();

    let start_ms = Timestamp::now().as_millis();

    for h in 1..=10u64 {
        let template = BlockBuilder::new()
            .height(BlockHeight::new(h))
            .parent_hash(parent.clone())
            .state_root(executor.state().state_root().clone())
            .miner(miner_addr.clone())
            .difficulty(1)
            .timestamp(Timestamp::from_millis(1_700_000_000_000 + h * 10_000))
            .build()
            .unwrap();

        let result = miner.mine(template, || false).unwrap();
        let block  = result.block;

        // Verify PoW.
        use block::BlockValidator;
        assert!(BlockValidator::validate_pow(&block).is_ok(),
            "block {} must meet PoW target", h);

        // Verify parent linkage.
        assert_eq!(block.parent_hash().as_bytes(), parent.as_bytes(),
            "block {} must reference correct parent", h);

        parent = block.hash();
        chain.add_block(block.clone()).unwrap();
        executor.execute_block(&block).unwrap();

        assert_eq!(chain.height().as_u64(), h);
        assert!(executor.state().verify_supply_invariant(),
            "supply invariant violated at height {}", h);
    }

    let elapsed = Timestamp::now().as_millis().saturating_sub(start_ms);
    let total_reward: u64 = (1..=10)
        .map(|h| block_reward_at(&BlockHeight::new(h)).as_micro())
        .sum();

    println!("[test_mine_10_blocks] elapsed={}ms", elapsed);
    println!("[test_mine_10_blocks] miner balance={}", executor.state().get_balance(&miner_addr));
    println!("[test_mine_10_blocks] expected rewards={}", total_reward);

    assert_eq!(
        executor.state().get_balance(&miner_addr).as_micro(),
        total_reward,
        "miner balance must equal sum of block rewards"
    );
    assert_eq!(chain.height().as_u64(), 10);
}

#[test]
fn test_block_hash_meets_target_difficulty_1() {
    use block::{difficulty_to_target, meets_target};

    let addr     = make_address();
    let miner    = Miner::new(addr.clone());
    let template = BlockBuilder::new()
        .height(BlockHeight::new(1))
        .parent_hash(sha256(b"p"))
        .state_root(sha256(b"s"))
        .miner(addr)
        .difficulty(1)
        .timestamp(Timestamp::now())
        .build()
        .unwrap();

    let result = miner.mine(template, || false).unwrap();
    let target = difficulty_to_target(1);
    assert!(meets_target(&result.block.hash(), &target));
}

#[test]
fn test_mining_is_cancellable() {
    use consensus::ConsensusError;

    let addr  = make_address();
    let miner = Miner::new(addr.clone());

    let template = BlockBuilder::new()
        .height(BlockHeight::new(1))
        .parent_hash(sha256(b"p"))
        .state_root(sha256(b"s"))
        .miner(addr)
        .difficulty(u64::MAX) // impossible difficulty
        .timestamp(Timestamp::now())
        .build()
        .unwrap();

    // Cancel immediately.
    let result = miner.mine(template, || true);
    assert!(matches!(result, Err(ConsensusError::MiningFailed)));
}

#[test]
fn test_block_reward_halving() {
    use primitives::constants::{HALVING_INTERVAL, INITIAL_BLOCK_REWARD};

    let r0 = block_reward_at(&BlockHeight::new(0)).as_micro();
    let r1 = block_reward_at(&BlockHeight::new(HALVING_INTERVAL)).as_micro();
    let r2 = block_reward_at(&BlockHeight::new(HALVING_INTERVAL * 2)).as_micro();

    assert_eq!(r0, INITIAL_BLOCK_REWARD);
    assert_eq!(r1, INITIAL_BLOCK_REWARD / 2);
    assert_eq!(r2, INITIAL_BLOCK_REWARD / 4);
}
