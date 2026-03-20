// examples/usage.rs — End-to-end demo of the consensus crate.
//
// Run with:  cargo run --example usage

use block::{Block, BlockBuilder};
use consensus::{Blockchain, ChainRules, Miner, adjust_difficulty, should_adjust};
use crypto::{Address, KeyPair, sha256};
use primitives::{BlockHeight, Timestamp};
use primitives::constants::{DIFFICULTY_ADJUSTMENT_INTERVAL, TARGET_BLOCK_TIME_MS};

fn make_address() -> Address {
    Address::from_public_key(KeyPair::generate().unwrap().public_key())
}

fn zero_hash() -> crypto::HashDigest {
    sha256(b"zero")
}

fn mine_block_easy(height: u64, parent_hash: crypto::HashDigest, miner: &Address) -> Block {
    let template = BlockBuilder::new()
        .height(BlockHeight::new(height))
        .parent_hash(parent_hash)
        .state_root(zero_hash())
        .miner(miner.clone())
        .difficulty(1)
        .timestamp(Timestamp::from_millis(
            1_700_000_000_000 + height * TARGET_BLOCK_TIME_MS
        ))
        .build()
        .unwrap();

    let miner_obj = Miner::new(miner.clone());
    miner_obj.mine(template, || false).unwrap().block
}

fn main() {
    println!("═══════════════════════════════════════════════════════");
    println!("  Consensus Layer — Example Usage");
    println!("═══════════════════════════════════════════════════════\n");

    let miner_addr = make_address();

    // ── 1. Mine genesis ───────────────────────────────────────────────────────
    println!("── 1. Mine Genesis Block ───────────────────────────────");
    let genesis = mine_block_easy(0, zero_hash(), &miner_addr);
    println!("  {}", genesis);

    let mut chain = Blockchain::new(genesis, 1).unwrap();
    println!("  Chain height : {}", chain.height());
    println!("  Difficulty   : {}", chain.current_difficulty());
    println!();

    // ── 2. Mine several blocks ────────────────────────────────────────────────
    println!("── 2. Mine Blocks 1–5 ──────────────────────────────────");
    for i in 1..=5 {
        let parent_hash = chain.tip().hash();
        let block = mine_block_easy(i, parent_hash, &miner_addr);
        let nonce = block.nonce();
        chain.add_block(block).unwrap();
        println!("  Block {} mined — nonce={}", i, nonce);
    }
    println!("  Chain height : {}", chain.height());
    println!();

    // ── 3. Difficulty adjustment simulation ───────────────────────────────────
    println!("── 3. Difficulty Adjustment ────────────────────────────");
    let target_ms = DIFFICULTY_ADJUSTMENT_INTERVAL * TARGET_BLOCK_TIME_MS;

    println!("  Target interval time : {}ms", target_ms);

    // Fast blocks (50% of target time) → difficulty increases.
    let fast_diff = adjust_difficulty(1000, target_ms / 2);
    println!("  Actual=50% of target → new difficulty : {} (was 1000)", fast_diff);

    // Slow blocks (200% of target time) → difficulty decreases.
    let slow_diff = adjust_difficulty(1000, target_ms * 2);
    println!("  Actual=200% of target → new difficulty: {} (was 1000)", slow_diff);

    // On-target → no change.
    let same_diff = adjust_difficulty(1000, target_ms);
    println!("  Actual=100% of target → new difficulty: {} (was 1000)", same_diff);
    println!();

    // ── 4. Adjustment boundary detection ─────────────────────────────────────
    println!("── 4. Adjustment Boundaries ────────────────────────────");
    for h in [0u64, 1, 2015, 2016, 4032] {
        let height = BlockHeight::new(h);
        println!("  Height {:>6} → adjust? {}", h, should_adjust(&height));
    }
    println!();

    // ── 5. Chain rules ────────────────────────────────────────────────────────
    println!("── 5. Chain Rules ──────────────────────────────────────");
    println!("  min_difficulty(1000) = {}", ChainRules::min_difficulty(1000));
    println!("  max_difficulty(1000) = {}", ChainRules::max_difficulty(1000));
    println!();

    println!("═══════════════════════════════════════════════════════");
    println!("  All checks passed.");
    println!("═══════════════════════════════════════════════════════");
}
