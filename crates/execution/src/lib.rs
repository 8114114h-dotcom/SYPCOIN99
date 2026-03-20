// lib.rs — Public API for the execution crate.
//
//   use execution::{Executor, BlockReceipt, TxReceipt, TxStatus};
//   use execution::ExecutionError;

mod error;
mod tx_executor;
mod block_executor;
mod executor;

pub use error::ExecutionError;
pub use executor::Executor;
pub use block_executor::{BlockExecutor, BlockReceipt};
pub use tx_executor::{TxExecutor, TxReceipt, TxStatus};

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use block::{Block, BlockBuilder};
    use crypto::{Address, KeyPair, sha256};
    use primitives::{Amount, BlockHeight, Nonce, Timestamp};
    use primitives::constants::MIN_TX_FEE_MICRO;
    use state::WorldState;
    use transaction::TransactionBuilder;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn make_address() -> Address {
        Address::from_public_key(KeyPair::generate().unwrap().public_key())
    }

    fn zero_hash() -> crypto::HashDigest {
        sha256(b"zero")
    }

    fn make_tx(
        kp:        KeyPair,
        to:        Address,
        amount:    u64,
        nonce:     u64,
    ) -> transaction::Transaction {
        TransactionBuilder::new()
            .from_keypair(kp)
            .to(to)
            .amount(Amount::from_micro(amount).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(nonce))
            .build()
            .unwrap()
    }

    fn empty_block(height: u64, parent: crypto::HashDigest, state_root: crypto::HashDigest) -> Block {
        BlockBuilder::new()
            .height(BlockHeight::new(height))
            .parent_hash(parent)
            .state_root(state_root)
            .miner(make_address())
            .difficulty(1)
            .timestamp(Timestamp::from_millis(1_700_000_000_000 + height * 10_000))
            .build()
            .unwrap()
    }

    fn seeded_state(addr: &Address, tokens: u64) -> WorldState {
        let mut state = WorldState::new();
        state.set_genesis_balance(addr.clone(), Amount::from_tokens(tokens).unwrap()).unwrap();
        state
    }

    // ── TxExecutor ────────────────────────────────────────────────────────────

    #[test]
    fn test_tx_execute_success() {
        let sender_kp    = KeyPair::generate().unwrap();
        let sender_addr  = Address::from_public_key(sender_kp.public_key());
        let recipient    = make_address();
        let miner        = make_address();
        let mut state    = seeded_state(&sender_addr, 100);

        let tx = make_tx(sender_kp, recipient.clone(), 10_000_000, 1);
        let receipt = TxExecutor::execute(&mut state, &tx, &miner, Timestamp::now()).unwrap();

        assert!(receipt.status.is_success());
        assert_eq!(receipt.fee.as_micro(), MIN_TX_FEE_MICRO);
        assert_eq!(
            state.get_balance(&recipient),
            Amount::from_micro(10_000_000).unwrap()
        );
    }

    #[test]
    fn test_tx_execute_insufficient_balance() {
        let sender_kp   = KeyPair::generate().unwrap();
        let sender_addr = Address::from_public_key(sender_kp.public_key());
        let miner       = make_address();
        let mut state   = seeded_state(&sender_addr, 1);

        // Try to send 999 tokens with only 1.
        let tx = make_tx(sender_kp, make_address(), 999_000_000, 1);
        let receipt = TxExecutor::execute(&mut state, &tx, &miner, Timestamp::now()).unwrap();

        assert!(!receipt.status.is_success());
        // Balance unchanged.
        assert_eq!(state.get_balance(&sender_addr), Amount::from_tokens(1).unwrap());
    }

    #[test]
    fn test_tx_execute_wrong_nonce() {
        let sender_kp   = KeyPair::generate().unwrap();
        let sender_addr = Address::from_public_key(sender_kp.public_key());
        let miner       = make_address();
        let mut state   = seeded_state(&sender_addr, 100);

        // Account nonce=0, but tx nonce=5 (wrong).
        let tx = make_tx(sender_kp, make_address(), 1_000_000, 5);
        let receipt = TxExecutor::execute(&mut state, &tx, &miner, Timestamp::now()).unwrap();

        assert!(!receipt.status.is_success());
    }

    // ── BlockExecutor ─────────────────────────────────────────────────────────

    #[test]
    fn test_execute_empty_block() {
        let mut state = WorldState::new();
        // Genesis at height 0.
        state.commit(BlockHeight::new(0));

        let block = empty_block(1, zero_hash(), zero_hash());
        let receipt = BlockExecutor::execute(&mut state, &block).unwrap();

        assert_eq!(receipt.block_height, BlockHeight::new(1));
        assert_eq!(receipt.txs_succeeded, 0);
        assert_eq!(receipt.txs_failed, 0);
        // Miner received block reward.
        assert!(!receipt.reward_paid.is_zero());
    }

    #[test]
    fn test_execute_block_with_transactions() {
        let sender_kp   = KeyPair::generate().unwrap();
        let sender_addr = Address::from_public_key(sender_kp.public_key());
        let recipient   = make_address();
        let miner_addr  = make_address();

        let mut state = WorldState::new();
        state.set_genesis_balance(sender_addr.clone(), Amount::from_tokens(100).unwrap()).unwrap();
        state.commit(BlockHeight::new(0));

        let tx = make_tx(sender_kp, recipient.clone(), 5_000_000, 1);

        let block = BlockBuilder::new()
            .height(BlockHeight::new(1))
            .parent_hash(zero_hash())
            .state_root(zero_hash())
            .miner(miner_addr.clone())
            .difficulty(1)
            .timestamp(Timestamp::from_millis(1_700_000_010_000))
            .transactions(vec![tx])
            .build()
            .unwrap();

        let receipt = BlockExecutor::execute(&mut state, &block).unwrap();

        assert_eq!(receipt.txs_succeeded, 1);
        assert_eq!(receipt.txs_failed, 0);
        assert!(!receipt.reward_paid.is_zero());
        assert_eq!(state.get_balance(&recipient), Amount::from_micro(5_000_000).unwrap());
        assert!(state.verify_supply_invariant());
    }

    #[test]
    fn test_execute_block_wrong_height() {
        let mut state = WorldState::new();
        state.commit(BlockHeight::new(0));

        // State at height 0, block claims height 5.
        let block = empty_block(5, zero_hash(), zero_hash());
        let result = BlockExecutor::execute(&mut state, &block);
        assert!(matches!(result, Err(ExecutionError::BlockHeightMismatch { .. })));
    }

    #[test]
    fn test_dry_run_does_not_mutate_state() {
        let mut state = WorldState::new();
        let miner_addr = make_address();
        state.set_genesis_balance(miner_addr.clone(), Amount::from_tokens(0).unwrap()).unwrap();
        state.commit(BlockHeight::new(0));

        let supply_before = state.total_supply();
        let root_before   = state.state_root().clone();

        let block = empty_block(1, zero_hash(), zero_hash());
        BlockExecutor::dry_run(&state, &block).unwrap();

        // State must be unchanged.
        assert_eq!(state.total_supply(), supply_before);
        assert_eq!(state.state_root().as_bytes(), root_before.as_bytes());
        assert_eq!(state.block_height(), BlockHeight::new(0));
    }

    // ── Executor facade ───────────────────────────────────────────────────────

    #[test]
    fn test_executor_execute_block() {
        let mut base = WorldState::new();
        base.commit(BlockHeight::new(0));

        let mut executor = Executor::new(base);
        let block = empty_block(1, zero_hash(), zero_hash());
        let receipt = executor.execute_block(&block).unwrap();

        assert_eq!(receipt.block_height, BlockHeight::new(1));
        assert_eq!(executor.state().block_height(), BlockHeight::new(1));
    }

    #[test]
    fn test_executor_snapshot_restore() {
        let mut base = WorldState::new();
        let addr     = make_address();
        base.set_genesis_balance(addr.clone(), Amount::from_tokens(50).unwrap()).unwrap();
        base.commit(BlockHeight::new(0));

        let mut executor = Executor::new(base);
        let snap = executor.snapshot();

        // Execute a block.
        let block = empty_block(1, zero_hash(), zero_hash());
        executor.execute_block(&block).unwrap();
        assert_eq!(executor.state().block_height(), BlockHeight::new(1));

        // Restore to pre-block state.
        executor.restore(snap);
        assert_eq!(executor.state().block_height(), BlockHeight::new(0));
        assert_eq!(executor.state().get_balance(&addr), Amount::from_tokens(50).unwrap());
    }

    #[test]
    fn test_supply_invariant_after_block_execution() {
        let sender_kp   = KeyPair::generate().unwrap();
        let sender_addr = Address::from_public_key(sender_kp.public_key());

        let mut state = WorldState::new();
        state.set_genesis_balance(sender_addr.clone(), Amount::from_tokens(100).unwrap()).unwrap();
        state.commit(BlockHeight::new(0));

        let tx = make_tx(sender_kp, make_address(), 10_000_000, 1);
        let block = BlockBuilder::new()
            .height(BlockHeight::new(1))
            .parent_hash(zero_hash())
            .state_root(zero_hash())
            .miner(make_address())
            .difficulty(1)
            .timestamp(Timestamp::from_millis(1_700_000_010_000))
            .transactions(vec![tx])
            .build()
            .unwrap();

        BlockExecutor::execute(&mut state, &block).unwrap();
        assert!(state.verify_supply_invariant(),
            "supply invariant must hold after block execution");
    }
}
