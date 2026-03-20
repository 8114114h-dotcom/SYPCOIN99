// examples/usage.rs — End-to-end demonstration of the primitives crate.
//
// Run with:
//   cargo run --example usage

use primitives::{
    Amount, BlockHeight, Nonce, Timestamp,
    block_reward_at, display_to_micro, is_supply_valid,
    micro_to_display, theoretical_supply_at,
};
use primitives::constants::*;

fn main() {
    println!("═══════════════════════════════════════════════════════");
    println!("  Blockchain Primitives — Example Usage");
    println!("═══════════════════════════════════════════════════════\n");

    // ── Chain constants ───────────────────────────────────────────────────────
    println!("── Chain Constants ─────────────────────────────────────");
    println!("  Chain ID             : {}", CHAIN_ID);
    println!("  Chain name           : {}", CHAIN_NAME);
    println!("  Max block size       : {} bytes ({} KB)", MAX_BLOCK_SIZE, MAX_BLOCK_SIZE / 1024);
    println!("  Target block time    : {}ms", TARGET_BLOCK_TIME_MS);
    println!("  Max txs per block    : {}", MAX_TX_PER_BLOCK);
    println!("  Decimal places       : {}", DECIMAL_PLACES);
    println!("  Max supply           : {} tokens", micro_to_display(MAX_SUPPLY_MICRO));
    println!("  Initial reward       : {} tokens", micro_to_display(INITIAL_BLOCK_REWARD));
    println!("  Halving interval     : {} blocks", HALVING_INTERVAL);
    println!();

    // ── Amount arithmetic ─────────────────────────────────────────────────────
    println!("── Amount Arithmetic ───────────────────────────────────");

    let balance   = Amount::from_tokens(100).unwrap();
    let fee       = Amount::from_micro(MIN_TX_FEE_MICRO).unwrap();
    let transfer  = Amount::from_tokens(50).unwrap();

    println!("  Balance              : {}", balance);
    println!("  Transfer             : {}", transfer);
    println!("  Fee                  : {}", fee);

    let total_deducted = transfer.checked_add(fee).unwrap();
    let remaining      = balance.checked_sub(total_deducted).unwrap();

    println!("  Total deducted       : {}", total_deducted);
    println!("  Remaining balance    : {}", remaining);

    // Parse from display string.
    let parsed_micro = display_to_micro("12.500000").unwrap();
    println!("  Parsed \"12.500000\"   : {} micro-tokens", parsed_micro);
    println!("  Formatted back       : {}", micro_to_display(parsed_micro));
    println!();

    // ── BlockHeight and halving schedule ──────────────────────────────────────
    println!("── Block Reward Schedule (Halving) ─────────────────────");

    let checkpoints: &[u64] = &[
        0,
        HALVING_INTERVAL - 1,
        HALVING_INTERVAL,
        HALVING_INTERVAL * 2,
        HALVING_INTERVAL * 4,
        HALVING_INTERVAL * 32,
        HALVING_INTERVAL * 64,
    ];

    for &h in checkpoints {
        let height = BlockHeight::new(h);
        let reward = block_reward_at(&height);
        println!("  Height {:>12}  →  reward: {} tokens", h, reward);
    }
    println!();

    // ── Theoretical supply ────────────────────────────────────────────────────
    println!("── Theoretical Supply at Key Heights ───────────────────");

    let supply_heights: &[u64] = &[1, 1_000, HALVING_INTERVAL, HALVING_INTERVAL * 2];
    for &h in supply_heights {
        let height = BlockHeight::new(h);
        let supply = theoretical_supply_at(&height).unwrap();
        println!("  At height {:>10}  →  supply: {} tokens", h, supply);
    }
    println!();

    // ── Timestamp ─────────────────────────────────────────────────────────────
    println!("── Timestamp ───────────────────────────────────────────");

    let now  = Timestamp::now();
    let past = Timestamp::from_millis(now.as_millis() - 10_000);

    println!("  Now                  : {}ms", now.as_millis());
    println!("  10s ago              : {}ms", past.as_millis());
    println!("  Elapsed since past   : {}ms", now.millis_since(&past).unwrap());

    let far_future = Timestamp::from_millis(now.as_millis() + 999_999);
    match far_future.validate_not_future(&now, MAX_FUTURE_BLOCK_TIME_MS) {
        Ok(())  => println!("  Far future timestamp : accepted (unexpected)"),
        Err(e)  => println!("  Far future timestamp : correctly rejected — {}", e),
    }
    println!();

    // ── Nonce ─────────────────────────────────────────────────────────────────
    println!("── Nonce (Replay Protection) ───────────────────────────");

    let mut nonce = Nonce::zero();
    for _ in 0..5 {
        let next = nonce.next().unwrap();
        println!("  {} → {}", nonce, next);
        nonce = next;
    }

    let n3 = Nonce::new(3);
    let n4 = Nonce::new(4);
    let n9 = Nonce::new(9);
    println!("  {} follows {} ? {}", n4, n3, n4.follows(&n3));   // true
    println!("  {} follows {} ? {}", n9, n3, n9.follows(&n3));   // false
    println!();

    // ── Supply validation ─────────────────────────────────────────────────────
    println!("── Supply Validation ───────────────────────────────────");
    println!("  MAX amount valid     : {}", is_supply_valid(&Amount::MAX));
    println!("  ZERO amount valid    : {}", is_supply_valid(&Amount::ZERO));
    println!();

    println!("═══════════════════════════════════════════════════════");
    println!("  All checks passed.");
    println!("═══════════════════════════════════════════════════════");
}
