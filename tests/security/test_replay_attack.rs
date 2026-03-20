// security/test_replay_attack.rs
// Verify that a confirmed transaction cannot be replayed.

use crypto::{Address, KeyPair};
use execution::TxExecutor;
use primitives::{Amount, BlockHeight, Nonce, Timestamp};
use primitives::constants::MIN_TX_FEE_MICRO;
use security::ReplayProtection;
use state::WorldState;
use transaction::{TransactionBuilder, TransactionValidator};

fn make_address() -> Address {
    Address::from_public_key(KeyPair::generate().unwrap().public_key())
}

#[test]
fn test_replay_protection_rejects_seen_tx() {
    let mut rp = ReplayProtection::with_defaults();

    let kp        = KeyPair::generate().unwrap();
    let from_addr = Address::from_public_key(kp.public_key());
    let to_addr   = make_address();

    let tx = TransactionBuilder::new()
        .from_keypair(kp)
        .to(to_addr)
        .amount(Amount::from_tokens(1).unwrap())
        .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
        .nonce(Nonce::new(1))
        .build()
        .unwrap();

    let tx_id = tx.tx_id().clone();

    // First time: not seen → pass.
    assert!(rp.check(&tx_id).is_ok());
    rp.mark_seen(tx_id.clone());

    // Second time: replay → fail.
    assert!(rp.check(&tx_id).is_err());
}

#[test]
fn test_nonce_prevents_replay_in_state() {
    let kp        = KeyPair::generate().unwrap();
    let from_addr = Address::from_public_key(kp.public_key());
    let to_addr   = make_address();
    let miner     = make_address();

    let mut state = WorldState::new();
    state.set_genesis_balance(from_addr.clone(), Amount::from_tokens(100).unwrap()).unwrap();
    state.commit(BlockHeight::new(0));

    // Build the same transaction twice (same nonce).
    let kp2 = KeyPair::generate().unwrap();
    let from2 = Address::from_public_key(kp2.public_key());
    let mut state2 = WorldState::new();
    state2.set_genesis_balance(from2.clone(), Amount::from_tokens(100).unwrap()).unwrap();
    state2.commit(BlockHeight::new(0));

    let tx = TransactionBuilder::new()
        .from_keypair(kp2)
        .to(to_addr.clone())
        .amount(Amount::from_tokens(10).unwrap())
        .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
        .nonce(Nonce::new(1))
        .build()
        .unwrap();

    // First execution: succeeds, nonce advances to 1.
    let r1 = TxExecutor::execute(&mut state2, &tx, &miner, Timestamp::now()).unwrap();
    assert!(r1.status.is_success());

    // Second execution: state nonce is now 1, tx nonce is 1 → wrong (expected 2).
    let r2 = TxExecutor::execute(&mut state2, &tx, &miner, Timestamp::now()).unwrap();
    assert!(!r2.status.is_success(), "replay via same nonce must fail");
}

#[test]
fn test_different_transactions_independent() {
    let mut rp = ReplayProtection::with_defaults();

    let kp1 = KeyPair::generate().unwrap();
    let kp2 = KeyPair::generate().unwrap();
    let to  = make_address();

    let tx1 = TransactionBuilder::new()
        .from_keypair(kp1)
        .to(to.clone())
        .amount(Amount::from_tokens(1).unwrap())
        .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
        .nonce(Nonce::new(1))
        .build()
        .unwrap();

    let tx2 = TransactionBuilder::new()
        .from_keypair(kp2)
        .to(to)
        .amount(Amount::from_tokens(2).unwrap())
        .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
        .nonce(Nonce::new(1))
        .build()
        .unwrap();

    rp.mark_seen(tx1.tx_id().clone());

    // tx2 is independent — must not be affected by tx1.
    assert!(rp.check(tx2.tx_id()).is_ok());
}
