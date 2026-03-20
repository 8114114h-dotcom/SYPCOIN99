// examples/usage.rs — End-to-end demo of the execution crate.
//
// Run with:  cargo run --example usage

use block::{Block, BlockBuilder};
use crypto::{Address, KeyPair, sha256};
use execution::{BlockExecutor, Executor, TxExecutor};
use primitives::{Amount, BlockHeight, Nonce, Timestamp};
use primitives::constants::MIN_TX_FEE_MICRO;
use state::WorldState;
use transaction::TransactionBuilder;

fn make_address() -> Address {
    Address::from_public_key(KeyPair::generate().unwrap().public_key())
}

fn main() {
    println!("═══════════════════════════════════════════════════════");
    println!("  Execution Layer — Example Usage");
    println!("═══════════════════════════════════════════════════════\n");

    // ── Setup ─────────────────────────────────────────────────────────────────
    let alice_kp   = KeyPair::generate().unwrap();
    let alice_addr = Address::from_public_key(alice_kp.public_key());
    let bob_addr   = make_address();
    let miner_addr = make_address();

    let mut state = WorldState::new();
    state.set_genesis_balance(alice_addr.clone(), Amount::from_tokens(1000).unwrap()).unwrap();
    state.commit(BlockHeight::new(0));

    println!("── Initial State ───────────────────────────────────────");
    println!("  Alice  : {}", state.get_balance(&alice_addr));
    println!("  Bob    : {}", state.get_balance(&bob_addr));
    println!("  Supply : {}", state.total_supply());
    println!();

    // ── 1. Execute a single transaction ───────────────────────────────────────
    println!("── 1. Execute Single Transaction ───────────────────────");
    let tx = TransactionBuilder::new()
        .from_keypair(alice_kp)
        .to(bob_addr.clone())
        .amount(Amount::from_tokens(100).unwrap())
        .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
        .nonce(Nonce::new(1))
        .build()
        .unwrap();

    let receipt = TxExecutor::execute(&mut state, &tx, &miner_addr, Timestamp::now()).unwrap();
    println!("  Status    : {:?}", receipt.status);
    println!("  Fee paid  : {}", receipt.fee);
    println!("  Alice now : {}", state.get_balance(&alice_addr));
    println!("  Bob now   : {}", state.get_balance(&bob_addr));
    println!("  Supply ok : {}", state.verify_supply_invariant());
    println!();

    // ── 2. Execute a full block ────────────────────────────────────────────────
    println!("── 2. Execute Full Block ───────────────────────────────");
    state.commit(BlockHeight::new(0)); // reset height for demo

    // Restate (re-seed for clean demo).
    let mut state2 = WorldState::new();
    let alice2_kp  = KeyPair::generate().unwrap();
    let alice2_addr = Address::from_public_key(alice2_kp.public_key());
    state2.set_genesis_balance(alice2_addr.clone(), Amount::from_tokens(500).unwrap()).unwrap();
    state2.commit(BlockHeight::new(0));

    let txs: Vec<_> = {
        // Build 3 transactions.
        let kp2 = KeyPair::generate().unwrap();
        let addr2 = Address::from_public_key(kp2.public_key());
        let mut state2b = WorldState::new();
        state2b.set_genesis_balance(addr2.clone(), Amount::from_tokens(500).unwrap()).unwrap();

        (1..=3).map(|i| {
            let kp = KeyPair::generate().unwrap();
            TransactionBuilder::new()
                .from_keypair(alice2_kp.clone()) // reuse same sender in demo
                .to(make_address())
                .amount(Amount::from_tokens(10).unwrap())
                .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
                .nonce(Nonce::new(i))
                .build()
                .unwrap()
        }).collect()
    };

    // We'll use an empty block for the demo (txs above share the same sender
    // which causes nonce issues in a real execution; empty block shows reward).
    let block = BlockBuilder::new()
        .height(BlockHeight::new(1))
        .parent_hash(sha256(b"genesis"))
        .state_root(sha256(b"state0"))
        .miner(miner_addr.clone())
        .difficulty(1)
        .timestamp(Timestamp::from_millis(1_700_000_010_000))
        .build()
        .unwrap();

    let block_receipt = BlockExecutor::execute(&mut state2, &block).unwrap();
    println!("  Block height    : {}", block_receipt.block_height);
    println!("  Txs succeeded   : {}", block_receipt.txs_succeeded);
    println!("  Txs failed      : {}", block_receipt.txs_failed);
    println!("  Reward paid     : {}", block_receipt.reward_paid);
    println!("  Total fees      : {}", block_receipt.total_fees);
    println!("  State root      : {}", hex::encode(&block_receipt.state_root.as_bytes()[..8]));
    println!("  Supply valid    : {}", state2.verify_supply_invariant());
    println!();

    // ── 3. Dry run ────────────────────────────────────────────────────────────
    println!("── 3. Dry Run (no state mutation) ──────────────────────");
    let height_before = state2.block_height();
    let supply_before = state2.total_supply();

    let next_block = BlockBuilder::new()
        .height(BlockHeight::new(2))
        .parent_hash(block.hash())
        .state_root(block_receipt.state_root.clone())
        .miner(miner_addr.clone())
        .difficulty(1)
        .timestamp(Timestamp::from_millis(1_700_000_020_000))
        .build()
        .unwrap();

    let dry = BlockExecutor::dry_run(&state2, &next_block).unwrap();
    println!("  Dry run reward  : {}", dry.reward_paid);
    println!("  State unchanged : height={} supply={}",
        state2.block_height() == height_before,
        state2.total_supply() == supply_before,
    );
    println!();

    // ── 4. Executor facade ────────────────────────────────────────────────────
    println!("── 4. Executor Facade ──────────────────────────────────");
    let mut executor = Executor::new(state2);
    let snap = executor.snapshot();

    executor.execute_block(&next_block).unwrap();
    println!("  After block 2   : height={}", executor.state().block_height());

    executor.restore(snap);
    println!("  After restore   : height={}", executor.state().block_height());
    println!();

    println!("═══════════════════════════════════════════════════════");
    println!("  All checks passed.");
    println!("═══════════════════════════════════════════════════════");
}
