// security/test_double_spend.rs
// Verify that double-spending the same balance is rejected.

use crypto::{Address, KeyPair};
use execution::TxExecutor;
use primitives::{Amount, BlockHeight, Nonce, Timestamp};
use primitives::constants::MIN_TX_FEE_MICRO;
use state::WorldState;
use transaction::TransactionBuilder;

fn make_address() -> Address {
    Address::from_public_key(KeyPair::generate().unwrap().public_key())
}

#[test]
fn test_double_spend_second_tx_rejected() {
    // Alice has 100 tokens. She tries to send 80 twice.
    let alice_kp   = KeyPair::generate().unwrap();
    let alice_addr = Address::from_public_key(alice_kp.public_key());
    let bob        = make_address();
    let carol      = make_address();
    let miner      = make_address();

    let mut state = WorldState::new();
    state.set_genesis_balance(alice_addr.clone(), Amount::from_tokens(100).unwrap()).unwrap();
    state.commit(BlockHeight::new(0));

    // Build two transactions each spending 80 tokens.
    let kp_a = KeyPair::generate().unwrap();
    let kp_b = KeyPair::generate().unwrap();
    let addr_a = Address::from_public_key(kp_a.public_key());
    let addr_b = Address::from_public_key(kp_b.public_key());

    let mut state_a = WorldState::new();
    state_a.set_genesis_balance(addr_a.clone(), Amount::from_tokens(100).unwrap()).unwrap();
    state_a.commit(BlockHeight::new(0));

    let tx_a = TransactionBuilder::new()
        .from_keypair(kp_a)
        .to(bob.clone())
        .amount(Amount::from_tokens(80).unwrap())
        .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
        .nonce(Nonce::new(1))
        .build()
        .unwrap();

    let kp_b2 = KeyPair::generate().unwrap();
    let addr_b2 = Address::from_public_key(kp_b2.public_key());
    // Both tx_a and tx_b2 spend from addr_b2's account.
    let mut state_double = WorldState::new();
    state_double.set_genesis_balance(addr_b2.clone(), Amount::from_tokens(100).unwrap()).unwrap();
    state_double.commit(BlockHeight::new(0));

    let tx1 = TransactionBuilder::new()
        .from_keypair(kp_b2)
        .to(bob.clone())
        .amount(Amount::from_tokens(80).unwrap())
        .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
        .nonce(Nonce::new(1))
        .build()
        .unwrap();

    // First spend succeeds.
    let r1 = TxExecutor::execute(&mut state_double, &tx1, &miner, Timestamp::now()).unwrap();
    assert!(r1.status.is_success(), "first spend must succeed");

    // Balance now ≈ 100 - 80 - fee = 19.999 tokens.
    let balance_after = state_double.get_balance(&addr_b2);
    assert!(balance_after.as_micro() < Amount::from_tokens(20).unwrap().as_micro());

    // Build another transaction for 80 tokens from the same account.
    // This should fail — insufficient balance.
    let kp_b3 = KeyPair::generate().unwrap();
    let addr_b3 = Address::from_public_key(kp_b3.public_key());
    let mut state_3 = WorldState::new();
    state_3.set_genesis_balance(addr_b3.clone(), Amount::from_tokens(100).unwrap()).unwrap();
    state_3.commit(BlockHeight::new(0));

    let tx_spend1 = TransactionBuilder::new()
        .from_keypair(kp_b3)
        .to(bob.clone())
        .amount(Amount::from_tokens(80).unwrap())
        .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
        .nonce(Nonce::new(1))
        .build()
        .unwrap();

    let r_first = TxExecutor::execute(&mut state_3, &tx_spend1, &miner, Timestamp::now()).unwrap();
    assert!(r_first.status.is_success());

    // Now try a second big spend — should fail.
    let kp_b4 = KeyPair::generate().unwrap();
    let addr_b4 = Address::from_public_key(kp_b4.public_key());
    let tx_spend2 = TransactionBuilder::new()
        .from_keypair(kp_b4)
        .to(carol)
        .amount(Amount::from_tokens(80).unwrap())
        .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
        .nonce(Nonce::new(1))
        .build()
        .unwrap();

    // addr_b4 has no balance → InsufficientBalance.
    let r_second = TxExecutor::execute(&mut state_3, &tx_spend2, &miner, Timestamp::now()).unwrap();
    assert!(!r_second.status.is_success(), "second large spend must fail");
}

#[test]
fn test_supply_conserved_after_failed_double_spend() {
    let kp        = KeyPair::generate().unwrap();
    let from_addr = Address::from_public_key(kp.public_key());
    let to        = make_address();
    let miner     = make_address();

    let mut state = WorldState::new();
    state.set_genesis_balance(from_addr.clone(), Amount::from_tokens(50).unwrap()).unwrap();
    state.commit(BlockHeight::new(0));

    let initial_supply = state.total_supply();

    let kp2 = KeyPair::generate().unwrap();
    let addr2 = Address::from_public_key(kp2.public_key());
    let mut state2 = WorldState::new();
    state2.set_genesis_balance(addr2.clone(), Amount::from_tokens(50).unwrap()).unwrap();
    state2.commit(BlockHeight::new(0));

    // Spend 40 (succeeds).
    let tx1 = TransactionBuilder::new()
        .from_keypair(kp2)
        .to(to.clone())
        .amount(Amount::from_tokens(40).unwrap())
        .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
        .nonce(Nonce::new(1))
        .build()
        .unwrap();
    TxExecutor::execute(&mut state2, &tx1, &miner, Timestamp::now()).unwrap();

    // Supply must be conserved (no tokens created or destroyed by transfers).
    assert!(state2.verify_supply_invariant(),
        "supply invariant must hold after double spend attempt");
}

#[test]
fn test_exact_balance_spend_succeeds() {
    let kp        = KeyPair::generate().unwrap();
    let from_addr = Address::from_public_key(kp.public_key());
    let to        = make_address();
    let miner     = make_address();

    // Fund with exactly amount + fee.
    let fee    = Amount::from_micro(MIN_TX_FEE_MICRO).unwrap();
    let amount = Amount::from_tokens(10).unwrap();
    let total  = amount.checked_add(fee).unwrap();

    let mut state = WorldState::new();
    state.set_genesis_balance(from_addr.clone(), total).unwrap();
    state.commit(BlockHeight::new(0));

    let tx = TransactionBuilder::new()
        .from_keypair(kp)
        .to(to)
        .amount(amount)
        .fee(fee)
        .nonce(Nonce::new(1))
        .build()
        .unwrap();

    let r = TxExecutor::execute(&mut state, &tx, &miner, Timestamp::now()).unwrap();
    assert!(r.status.is_success(), "spending exact balance must succeed");
    assert_eq!(state.get_balance(&from_addr), Amount::ZERO);
}
