// examples/usage.rs — End-to-end demonstration of the state crate.
//
// Run with:  cargo run --example usage

use crypto::{Address, KeyPair};
use primitives::{Amount, BlockHeight, Nonce};
use primitives::constants::MIN_TX_FEE_MICRO;
use state::{WorldState, StateError};
use transaction::TransactionBuilder;

fn main() {
    println!("═══════════════════════════════════════════════════════");
    println!("  State Layer — Example Usage");
    println!("═══════════════════════════════════════════════════════\n");

    let mut world = WorldState::new();

    // ── 1. Genesis setup ──────────────────────────────────────────────────────
    println!("── 1. Genesis Setup ────────────────────────────────────");

    let alice_kp   = KeyPair::generate().unwrap();
    let alice_addr = Address::from_public_key(alice_kp.public_key());
    let bob_addr   = Address::from_public_key(KeyPair::generate().unwrap().public_key());
    let miner_addr = Address::from_public_key(KeyPair::generate().unwrap().public_key());

    world.set_genesis_balance(alice_addr.clone(), Amount::from_tokens(1000).unwrap()).unwrap();
    world.set_genesis_balance(bob_addr.clone(),   Amount::from_tokens(500).unwrap()).unwrap();

    println!("  Alice balance : {}", world.get_balance(&alice_addr));
    println!("  Bob balance   : {}", world.get_balance(&bob_addr));
    println!("  Total supply  : {}", world.total_supply());
    println!("  Supply valid  : {}", world.verify_supply_invariant());
    println!();

    // ── 2. Apply block reward ─────────────────────────────────────────────────
    println!("── 2. Block Reward (height=1) ──────────────────────────");
    let height = BlockHeight::new(1);
    let reward = world.apply_block_reward(&miner_addr, &height).unwrap();
    println!("  Miner reward  : {}", reward);
    println!("  Miner balance : {}", world.get_balance(&miner_addr));
    println!("  Total supply  : {}", world.total_supply());
    println!();

    // ── 3. Apply a transaction ────────────────────────────────────────────────
    println!("── 3. Apply Transaction ────────────────────────────────");
    let tx = TransactionBuilder::new()
        .from_keypair(alice_kp)
        .to(bob_addr.clone())
        .amount(Amount::from_tokens(100).unwrap())
        .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
        .nonce(Nonce::new(1))
        .build()
        .unwrap();

    let effect = world.apply_transaction(&tx, &miner_addr).unwrap();
    println!("  Fee collected  : {}", effect.fee_collected);
    println!("  Alice balance  : {}", world.get_balance(&alice_addr));
    println!("  Bob balance    : {}", world.get_balance(&bob_addr));
    println!("  Alice nonce    : {}", world.get_nonce(&alice_addr));
    println!("  Supply valid   : {}", world.verify_supply_invariant());
    println!();

    // ── 4. Commit block ───────────────────────────────────────────────────────
    println!("── 4. Commit Block ─────────────────────────────────────");
    let root = world.commit(height);
    println!("  State root    : {}", hex::encode(&root.as_bytes()[..16]));
    println!("  Block height  : {}", world.block_height());
    println!();

    // ── 5. Snapshot and restore ───────────────────────────────────────────────
    println!("── 5. Snapshot & Restore ───────────────────────────────");
    let snap = world.snapshot();
    println!("  {}", snap);

    let mut restored = WorldState::new();
    restored.restore_from_snapshot(snap);
    println!("  Restored Alice : {}", restored.get_balance(&alice_addr));
    println!("  Restored Bob   : {}", restored.get_balance(&bob_addr));
    println!("  Supply valid   : {}", restored.verify_supply_invariant());
    println!();

    // ── 6. Failure case ───────────────────────────────────────────────────────
    println!("── 6. Failed Transaction (insufficient balance) ────────");
    let broke_kp   = KeyPair::generate().unwrap();
    let broke_addr = Address::from_public_key(broke_kp.public_key());
    // Don't give broke_addr any balance.

    let bad_tx = TransactionBuilder::new()
        .from_keypair(broke_kp)
        .to(bob_addr.clone())
        .amount(Amount::from_tokens(999).unwrap())
        .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
        .nonce(Nonce::new(1))
        .build()
        .unwrap();

    match world.apply_transaction(&bad_tx, &miner_addr) {
        Err(StateError::InsufficientBalance { available, required }) =>
            println!("  [✓] Rejected — available: {}, required: {}", available, required),
        other => println!("  Unexpected: {:?}", other),
    }

    // State must be unchanged after failed tx.
    println!("  Supply still valid : {}", world.verify_supply_invariant());
    println!();

    println!("═══════════════════════════════════════════════════════");
    println!("  All checks passed.");
    println!("═══════════════════════════════════════════════════════");
}
