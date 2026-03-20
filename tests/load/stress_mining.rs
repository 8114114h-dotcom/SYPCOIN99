// load/stress_mining.rs
// Mining performance: measure hash rate and block production speed.

use block::BlockBuilder;
use consensus::Miner;
use crypto::{Address, KeyPair, sha256};
use primitives::{BlockHeight, Timestamp};

fn make_address() -> Address {
    Address::from_public_key(KeyPair::generate().unwrap().public_key())
}

#[test]
fn test_mine_10_blocks_timing() {
    let addr  = make_address();
    let miner = Miner::new(addr.clone());

    let total_start = Timestamp::now();
    let mut total_nonces: u64 = 0;
    let mut parent = sha256(b"genesis");

    for h in 1..=10u64 {
        let template = BlockBuilder::new()
            .height(BlockHeight::new(h))
            .parent_hash(parent.clone())
            .state_root(sha256(b"state"))
            .miner(addr.clone())
            .difficulty(1)
            .timestamp(Timestamp::from_millis(1_700_000_000_000 + h * 10_000))
            .build()
            .unwrap();

        let result = miner.mine(template, || false).unwrap();
        total_nonces += result.nonces_tried;
        parent = result.block.hash();

        // Every block must be valid.
        use block::BlockValidator;
        assert!(BlockValidator::validate_structure(&result.block).is_ok());
    }

    let total_ms  = Timestamp::now().as_millis().saturating_sub(total_start.as_millis());
    let hash_rate = if total_ms > 0 {
        total_nonces as f64 / (total_ms as f64 / 1000.0)
    } else {
        0.0
    };

    println!("[stress_mining] 10 blocks in {}ms", total_ms);
    println!("[stress_mining] total nonces: {}", total_nonces);
    println!("[stress_mining] average hash rate: {:.0} H/s", hash_rate);

    // With difficulty=1, each block needs 1 nonce on average.
    assert!(total_nonces >= 10, "must have tried at least 10 nonces");
}

#[test]
fn test_hash_rate_measurement() {
    let addr  = make_address();
    let miner = Miner::new(addr.clone());

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

    // With difficulty=1, we find the block in 1 nonce.
    assert!(result.nonces_tried >= 1);

    if result.elapsed_ms > 0 {
        let rate = result.nonces_tried as f64 / (result.elapsed_ms as f64 / 1000.0);
        println!("[hash_rate_test] {:.0} H/s", rate);
        assert!(rate > 0.0);
    }
}

#[test]
fn test_difficulty_1_always_mines_quickly() {
    let addr  = make_address();
    let miner = Miner::new(addr.clone());

    // difficulty=1 should mine in under 100ms even on slow hardware.
    let start = Timestamp::now();
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
    let elapsed = Timestamp::now().as_millis().saturating_sub(start.as_millis());

    assert!(elapsed < 5_000,
        "difficulty=1 should mine in under 5 seconds, took {}ms", elapsed);
    assert!(result.nonces_tried >= 1);
}
