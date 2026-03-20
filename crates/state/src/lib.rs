#![allow(dead_code, unused_imports)]
// lib.rs — Public API surface for the `state` crate.
//
// Downstream crates import ONLY from here:
#![allow(dead_code)]
//
//   use state::{WorldState, StateSnapshot, StateError};
//   use state::TxEffect;

mod error;
mod cache;
mod journal;

mod account {
    pub(crate) mod account;
    pub(crate) mod store;
}

mod trie {
    pub(crate) mod merkle_trie;
}

mod state {
    pub(crate) mod snapshot;
    pub(crate) mod transition;
    pub(crate) mod world_state;
}

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use error::StateError;
pub use state::world_state::WorldState;
pub use state::snapshot::StateSnapshot;
pub use state::transition::TxEffect;

// Account is exposed for RPC / storage layers that need to read account data.
pub use account::account::Account;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crypto::{Address, KeyPair};
    use primitives::{Amount, BlockHeight, Nonce};
    use primitives::constants::MIN_TX_FEE_MICRO;
    use transaction::TransactionBuilder;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn make_address() -> Address {
        let kp = KeyPair::generate().unwrap();
        Address::from_public_key(kp.public_key())
    }

    fn make_tx(
        sender_kp: KeyPair,
        recipient: Address,
        amount:    u64,
        nonce:     u64,
    ) -> transaction::Transaction {
        TransactionBuilder::new()
            .from_keypair(sender_kp)
            .to(recipient)
            .amount(Amount::from_micro(amount).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(nonce))
            .build()
            .unwrap()
    }

    // ── WorldState basics ─────────────────────────────────────────────────────

    #[test]
    fn test_new_state_is_empty() {
        let state = WorldState::new();
        assert_eq!(state.account_count(), 0);
        assert_eq!(state.total_supply(), Amount::ZERO);
        assert!(state.block_height().is_genesis());
    }

    #[test]
    fn test_get_balance_nonexistent_returns_zero() {
        let state = WorldState::new();
        let addr  = make_address();
        assert_eq!(state.get_balance(&addr), Amount::ZERO);
    }

    #[test]
    fn test_get_nonce_nonexistent_returns_zero() {
        let state = WorldState::new();
        let addr  = make_address();
        assert_eq!(state.get_nonce(&addr), Nonce::ZERO);
    }

    // ── Genesis balances ──────────────────────────────────────────────────────

    #[test]
    fn test_set_genesis_balance() {
        let mut state = WorldState::new();
        let addr      = make_address();
        let amount    = Amount::from_tokens(1000).unwrap();

        state.set_genesis_balance(addr.clone(), amount).unwrap();

        assert_eq!(state.get_balance(&addr), amount);
        assert_eq!(state.total_supply(), amount);
    }

    #[test]
    fn test_genesis_balance_rejected_after_block_commit() {
        let mut state = WorldState::new();
        state.commit(BlockHeight::new(1));

        let result = state.set_genesis_balance(
            make_address(),
            Amount::from_tokens(100).unwrap(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_supply_invariant_holds_after_genesis() {
        let mut state = WorldState::new();
        let addr1 = make_address();
        let addr2 = make_address();

        state.set_genesis_balance(addr1, Amount::from_tokens(500).unwrap()).unwrap();
        state.set_genesis_balance(addr2, Amount::from_tokens(300).unwrap()).unwrap();

        assert!(state.verify_supply_invariant());
        assert_eq!(state.total_supply(), Amount::from_tokens(800).unwrap());
    }

    // ── Transaction application ───────────────────────────────────────────────

    #[test]
    fn test_apply_transaction_transfers_balance() {
        let mut state     = WorldState::new();
        let sender_kp     = KeyPair::generate().unwrap();
        let sender_addr   = Address::from_public_key(sender_kp.public_key());
        let recipient_addr = make_address();
        let miner_addr     = make_address();

        // Seed sender with 100 tokens.
        state.set_genesis_balance(
            sender_addr.clone(),
            Amount::from_tokens(100).unwrap(),
        ).unwrap();

        let tx = make_tx(sender_kp, recipient_addr.clone(), 10_000_000, 1);
        let effect = state.apply_transaction(&tx, &miner_addr).unwrap();

        // Recipient received 10 tokens.
        assert_eq!(
            state.get_balance(&recipient_addr),
            Amount::from_micro(10_000_000).unwrap()
        );
        // Miner received the fee.
        assert_eq!(
            state.get_balance(&miner_addr),
            Amount::from_micro(MIN_TX_FEE_MICRO).unwrap()
        );
        // Sender lost amount + fee.
        let expected_sender = Amount::from_tokens(100).unwrap()
            .checked_sub(Amount::from_micro(10_000_000 + MIN_TX_FEE_MICRO).unwrap())
            .unwrap();
        assert_eq!(state.get_balance(&sender_addr), expected_sender);
        assert_eq!(effect.fee_collected.as_micro(), MIN_TX_FEE_MICRO);
    }

    #[test]
    fn test_apply_transaction_increments_sender_nonce() {
        let mut state    = WorldState::new();
        let sender_kp    = KeyPair::generate().unwrap();
        let sender_addr  = Address::from_public_key(sender_kp.public_key());
        let miner_addr   = make_address();

        state.set_genesis_balance(
            sender_addr.clone(),
            Amount::from_tokens(100).unwrap(),
        ).unwrap();

        let tx = make_tx(sender_kp, make_address(), 1_000_000, 1);
        state.apply_transaction(&tx, &miner_addr).unwrap();

        assert_eq!(state.get_nonce(&sender_addr), Nonce::new(1));
    }

    #[test]
    fn test_apply_transaction_insufficient_balance_fails() {
        let mut state   = WorldState::new();
        let sender_kp   = KeyPair::generate().unwrap();
        let sender_addr = Address::from_public_key(sender_kp.public_key());
        let miner_addr  = make_address();

        // Only 1 token but trying to send 50.
        state.set_genesis_balance(
            sender_addr.clone(),
            Amount::from_tokens(1).unwrap(),
        ).unwrap();

        let tx = make_tx(sender_kp, make_address(), 50_000_000, 1);
        let result = state.apply_transaction(&tx, &miner_addr);
        assert!(matches!(result, Err(StateError::InsufficientBalance { .. })));

        // Balance must be unchanged after failed tx.
        assert_eq!(state.get_balance(&sender_addr), Amount::from_tokens(1).unwrap());
    }

    #[test]
    fn test_supply_invariant_after_transaction() {
        let mut state    = WorldState::new();
        let sender_kp    = KeyPair::generate().unwrap();
        let sender_addr  = Address::from_public_key(sender_kp.public_key());
        let miner_addr   = make_address();

        state.set_genesis_balance(
            sender_addr.clone(),
            Amount::from_tokens(100).unwrap(),
        ).unwrap();

        let tx = make_tx(sender_kp, make_address(), 10_000_000, 1);
        state.apply_transaction(&tx, &miner_addr).unwrap();

        // Supply must be conserved — transfers don't create or destroy tokens.
        assert!(state.verify_supply_invariant());
    }

    // ── Block reward ──────────────────────────────────────────────────────────

    #[test]
    fn test_apply_block_reward_increases_supply() {
        let mut state  = WorldState::new();
        let miner_addr = make_address();
        let height     = BlockHeight::new(1);

        let reward = state.apply_block_reward(&miner_addr, &height).unwrap();
        assert!(!reward.is_zero());
        assert_eq!(state.get_balance(&miner_addr), reward);
        assert_eq!(state.total_supply(), reward);
        assert!(state.verify_supply_invariant());
    }

    #[test]
    fn test_block_reward_zero_after_max_halvings() {
        let mut state  = WorldState::new();
        let miner_addr = make_address();
        // Height far beyond max halvings.
        let height = BlockHeight::new(210_000 * 65);
        let reward = state.apply_block_reward(&miner_addr, &height).unwrap();
        assert_eq!(reward, Amount::ZERO);
    }

    // ── Commit and state root ─────────────────────────────────────────────────

    #[test]
    fn test_commit_advances_height() {
        let mut state = WorldState::new();
        state.commit(BlockHeight::new(1));
        assert_eq!(state.block_height(), BlockHeight::new(1));
    }

    #[test]
    fn test_state_root_changes_after_mutation() {
        let mut state = WorldState::new();
        let root_before = state.state_root().clone();

        state.set_genesis_balance(make_address(), Amount::from_tokens(1).unwrap()).unwrap();
        state.commit(BlockHeight::new(1));

        assert_ne!(state.state_root().as_bytes(), root_before.as_bytes());
    }

    #[test]
    fn test_state_root_deterministic() {
        let addr   = make_address();
        let amount = Amount::from_tokens(42).unwrap();

        let mut s1 = WorldState::new();
        s1.set_genesis_balance(addr.clone(), amount).unwrap();
        s1.commit(BlockHeight::new(1));

        let mut s2 = WorldState::new();
        s2.set_genesis_balance(addr, amount).unwrap();
        s2.commit(BlockHeight::new(1));

        assert_eq!(s1.state_root().as_bytes(), s2.state_root().as_bytes());
    }

    // ── Snapshot ──────────────────────────────────────────────────────────────

    #[test]
    fn test_snapshot_and_restore() {
        let mut state = WorldState::new();
        let addr      = make_address();
        let amount    = Amount::from_tokens(77).unwrap();

        state.set_genesis_balance(addr.clone(), amount).unwrap();
        state.commit(BlockHeight::new(5));

        let snap = state.snapshot();
        assert_eq!(snap.block_height, BlockHeight::new(5));
        assert_eq!(snap.total_supply, amount);

        // Restore into a fresh state.
        let mut restored = WorldState::new();
        restored.restore_from_snapshot(snap);

        assert_eq!(restored.get_balance(&addr), amount);
        assert_eq!(restored.block_height(), BlockHeight::new(5));
        assert_eq!(restored.total_supply(), amount);
        assert!(restored.verify_supply_invariant());
    }
}
