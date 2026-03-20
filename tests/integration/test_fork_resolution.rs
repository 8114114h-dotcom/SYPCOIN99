// integration/test_fork_resolution.rs
// Fork detection and chain selection via cumulative work (heaviest chain rule).

use block::BlockBuilder;
use consensus::{Blockchain, evaluate_reorg, Miner, ReorgDecision};
use crypto::{Address, KeyPair, sha256};
use primitives::{BlockHeight, Timestamp};

fn make_address() -> Address {
    Address::from_public_key(KeyPair::generate().unwrap().public_key())
}

fn mine_block(height: u64, parent: crypto::HashDigest, difficulty: u64, addr: &Address) -> block::Block {
    let template = BlockBuilder::new()
        .height(BlockHeight::new(height))
        .parent_hash(parent)
        .state_root(sha256(b"state"))
        .miner(addr.clone())
        .difficulty(difficulty)
        .timestamp(Timestamp::from_millis(1_700_000_000_000 + height * 10_000))
        .build()
        .unwrap();

    let miner = Miner::new(addr.clone());
    miner.mine(template, || false).unwrap().block
}

#[test]
fn test_higher_difficulty_wins_fork() {
    let addr = make_address();

    // Fork A: difficulty=100
    let block_a = BlockBuilder::new()
        .height(BlockHeight::new(1))
        .parent_hash(sha256(b"genesis"))
        .state_root(sha256(b"state"))
        .miner(addr.clone())
        .difficulty(100)
        .timestamp(Timestamp::from_millis(1_700_000_010_000))
        .build()
        .unwrap();

    // Fork B: difficulty=50
    let block_b = BlockBuilder::new()
        .height(BlockHeight::new(1))
        .parent_hash(sha256(b"genesis"))
        .state_root(sha256(b"state"))
        .miner(addr.clone())
        .difficulty(50)
        .timestamp(Timestamp::from_millis(1_700_000_010_001))
        .build()
        .unwrap();

    // Chain A has more cumulative work → should win.
    let decision = evaluate_reorg(
        &[block_b.header()],    // current chain
        &[block_a.header()],    // candidate (higher difficulty)
    ).unwrap();

    assert_eq!(decision, ReorgDecision::Reorg,
        "higher difficulty chain should trigger reorg");
}

#[test]
fn test_lower_difficulty_loses_fork() {
    let addr = make_address();

    let strong = BlockBuilder::new()
        .height(BlockHeight::new(1))
        .parent_hash(sha256(b"genesis"))
        .state_root(sha256(b"state"))
        .miner(addr.clone())
        .difficulty(1000)
        .timestamp(Timestamp::from_millis(1_700_000_010_000))
        .build()
        .unwrap();

    let weak = BlockBuilder::new()
        .height(BlockHeight::new(1))
        .parent_hash(sha256(b"genesis"))
        .state_root(sha256(b"state"))
        .miner(addr.clone())
        .difficulty(10)
        .timestamp(Timestamp::from_millis(1_700_000_010_001))
        .build()
        .unwrap();

    let decision = evaluate_reorg(
        &[strong.header()], // current chain (strong)
        &[weak.header()],   // candidate (weak)
    ).unwrap();

    assert_eq!(decision, ReorgDecision::KeepCurrent,
        "weaker chain should not trigger reorg");
}

#[test]
fn test_longer_chain_same_difficulty_wins() {
    let addr = make_address();

    // Current: 1 block of difficulty 100.
    let single = BlockBuilder::new()
        .height(BlockHeight::new(1))
        .parent_hash(sha256(b"genesis"))
        .state_root(sha256(b"state"))
        .miner(addr.clone())
        .difficulty(100)
        .timestamp(Timestamp::from_millis(1_700_000_010_000))
        .build()
        .unwrap();

    // Candidate: 2 blocks of difficulty 60 each → cumulative 120 > 100.
    let b1 = BlockBuilder::new()
        .height(BlockHeight::new(1))
        .parent_hash(sha256(b"genesis"))
        .state_root(sha256(b"state"))
        .miner(addr.clone())
        .difficulty(60)
        .timestamp(Timestamp::from_millis(1_700_000_010_000))
        .build()
        .unwrap();

    let b2 = BlockBuilder::new()
        .height(BlockHeight::new(2))
        .parent_hash(b1.hash())
        .state_root(sha256(b"state"))
        .miner(addr.clone())
        .difficulty(60)
        .timestamp(Timestamp::from_millis(1_700_000_020_000))
        .build()
        .unwrap();

    let decision = evaluate_reorg(
        &[single.header()],              // current: 100
        &[b1.header(), b2.header()],     // candidate: 120
    ).unwrap();

    assert_eq!(decision, ReorgDecision::Reorg);
}

#[test]
fn test_reorg_depth_limit() {
    use consensus::validate_reorg_depth;
    use consensus::MAX_REORG_DEPTH;

    // Exactly at limit → OK.
    assert!(validate_reorg_depth(0, MAX_REORG_DEPTH).is_ok());
    // One over → error.
    assert!(validate_reorg_depth(0, MAX_REORG_DEPTH + 1).is_err());
}

#[test]
fn test_blockchain_rejects_duplicate() {
    let genesis = mine_block(0, sha256(b"p"), 1, &make_address());
    let hash    = genesis.hash();
    let mut chain = Blockchain::new(genesis, 1).unwrap();

    let b1 = mine_block(1, hash, 1, &make_address());
    chain.add_block(b1.clone()).unwrap();

    // Adding the same block again must fail.
    let result = chain.add_block(b1);
    assert!(result.is_err());
}
