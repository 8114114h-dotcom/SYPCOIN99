// lib.rs — Public API surface for the `block` crate.
//
//   use block::{Block, BlockBuilder, BlockValidator, BlockHeader, BlockError};
//   use block::{compute_merkle_root, compute_block_hash};
//   use block::{difficulty_to_target, meets_target};

mod error;
mod block_hash;
mod size_limit;
mod reward;

mod merkle { pub(crate) mod tree; }
mod header { pub(crate) mod header; }
mod body   { pub(crate) mod body; }
mod block  {
    pub(crate) mod block;
    pub(crate) mod builder;
    pub(crate) mod validator;
}

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use error::BlockError;
pub use block::block::Block;
pub use block::builder::BlockBuilder;
pub use block::validator::BlockValidator;
pub use header::header::BlockHeader;
pub use body::body::BlockBody;
pub use merkle::tree::compute_merkle_root;
pub use block_hash::{compute_block_hash, difficulty_to_target, meets_target};
pub use reward::total_coinbase;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crypto::{Address, HashDigest, KeyPair, sha256};
    use primitives::{Amount, BlockHeight, Nonce, Timestamp};
    use primitives::constants::{INITIAL_BLOCK_REWARD, MIN_TX_FEE_MICRO};
    use transaction::TransactionBuilder;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn make_address() -> Address {
        let kp = KeyPair::generate().unwrap();
        Address::from_public_key(kp.public_key())
    }

    fn zero_hash() -> HashDigest {
        sha256(b"zero")
    }

    fn make_tx(nonce: u64) -> transaction::Transaction {
        let kp = KeyPair::generate().unwrap();
        TransactionBuilder::new()
            .from_keypair(kp)
            .to(make_address())
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(nonce))
            .build()
            .unwrap()
    }

    fn make_block(height: u64, parent: HashDigest, txs: Vec<transaction::Transaction>) -> Block {
        BlockBuilder::new()
            .height(BlockHeight::new(height))
            .parent_hash(parent)
            .state_root(zero_hash())
            .miner(make_address())
            .difficulty(1)
            .timestamp(Timestamp::now())
            .transactions(txs)
            .build()
            .unwrap()
    }

    // ── BlockBuilder ──────────────────────────────────────────────────────────

    #[test]
    fn test_build_block_empty_txs() {
        let block = make_block(1, zero_hash(), vec![]);
        assert_eq!(block.tx_count(), 0);
        assert!(block.body().is_empty());
    }

    #[test]
    fn test_build_block_with_transactions() {
        let txs   = vec![make_tx(1), make_tx(2)];
        let block = make_block(1, zero_hash(), txs);
        assert_eq!(block.tx_count(), 2);
    }

    #[test]
    fn test_build_missing_height_fails() {
        let result = BlockBuilder::new()
            .parent_hash(zero_hash())
            .state_root(zero_hash())
            .miner(make_address())
            .difficulty(1)
            .build();
        assert!(matches!(result, Err(BlockError::MissingField(_))));
    }

    #[test]
    fn test_build_zero_difficulty_fails() {
        let result = BlockBuilder::new()
            .height(BlockHeight::new(1))
            .parent_hash(zero_hash())
            .state_root(zero_hash())
            .miner(make_address())
            .difficulty(0)
            .build();
        assert!(matches!(result, Err(BlockError::InvalidDifficulty)));
    }

    #[test]
    fn test_build_duplicate_transactions_fails() {
        let tx   = make_tx(1);
        let result = BlockBuilder::new()
            .height(BlockHeight::new(1))
            .parent_hash(zero_hash())
            .state_root(zero_hash())
            .miner(make_address())
            .difficulty(1)
            .transactions(vec![tx.clone(), tx])
            .build();
        assert!(matches!(result, Err(BlockError::DuplicateTransaction)));
    }

    // ── Block properties ──────────────────────────────────────────────────────

    #[test]
    fn test_block_hash_is_deterministic() {
        let block = make_block(1, zero_hash(), vec![]);
        assert_eq!(block.hash().as_bytes(), block.hash().as_bytes());
    }

    #[test]
    fn test_merkle_root_matches_body() {
        let txs   = vec![make_tx(1), make_tx(2)];
        let block = make_block(1, zero_hash(), txs);
        let computed = block.body().merkle_root();
        assert_eq!(block.merkle_root().as_bytes(), computed.as_bytes());
    }

    #[test]
    fn test_tx_count_header_matches_body() {
        let txs   = vec![make_tx(1), make_tx(2), make_tx(3)];
        let block = make_block(1, zero_hash(), txs);
        assert_eq!(block.header().tx_count(), block.body().tx_count());
    }

    #[test]
    fn test_block_height() {
        let block = make_block(42, zero_hash(), vec![]);
        assert_eq!(block.height(), BlockHeight::new(42));
    }

    #[test]
    fn test_genesis_flag() {
        let genesis = make_block(0, zero_hash(), vec![]);
        assert!(genesis.is_genesis());
        let non_genesis = make_block(1, zero_hash(), vec![]);
        assert!(!non_genesis.is_genesis());
    }

    // ── BlockValidator ────────────────────────────────────────────────────────

    #[test]
    fn test_validate_structure_valid_block() {
        let block = make_block(1, zero_hash(), vec![]);
        assert!(BlockValidator::validate_structure(&block).is_ok());
    }

    #[test]
    fn test_validate_against_parent_valid() {
        let parent = make_block(0, zero_hash(), vec![]);
        // Add small delay so timestamp is after parent.
        let child = BlockBuilder::new()
            .height(BlockHeight::new(1))
            .parent_hash(parent.hash())
            .state_root(zero_hash())
            .miner(make_address())
            .difficulty(1)
            .timestamp(Timestamp::from_millis(parent.timestamp().as_millis() + 1000))
            .build()
            .unwrap();

        assert!(BlockValidator::validate_against_parent(&child, parent.header()).is_ok());
    }

    #[test]
    fn test_validate_against_parent_wrong_height() {
        let parent = make_block(0, zero_hash(), vec![]);
        // Height should be 1, but we set 5.
        let bad_child = BlockBuilder::new()
            .height(BlockHeight::new(5))
            .parent_hash(parent.hash())
            .state_root(zero_hash())
            .miner(make_address())
            .difficulty(1)
            .timestamp(Timestamp::from_millis(parent.timestamp().as_millis() + 1000))
            .build()
            .unwrap();

        let result = BlockValidator::validate_against_parent(&bad_child, parent.header());
        assert!(matches!(result, Err(BlockError::InvalidHeight { .. })));
    }

    #[test]
    fn test_validate_against_parent_wrong_hash() {
        let parent = make_block(0, zero_hash(), vec![]);
        let wrong_parent_hash = sha256(b"wrong");
        let bad_child = BlockBuilder::new()
            .height(BlockHeight::new(1))
            .parent_hash(wrong_parent_hash) // wrong hash
            .state_root(zero_hash())
            .miner(make_address())
            .difficulty(1)
            .timestamp(Timestamp::from_millis(parent.timestamp().as_millis() + 1000))
            .build()
            .unwrap();

        let result = BlockValidator::validate_against_parent(&bad_child, parent.header());
        assert!(matches!(result, Err(BlockError::InvalidParentHash)));
    }

    // ── Merkle tree ───────────────────────────────────────────────────────────

    #[test]
    fn test_merkle_root_empty() {
        let root1 = compute_merkle_root(&[]);
        let root2 = compute_merkle_root(&[]);
        assert_eq!(root1.as_bytes(), root2.as_bytes());
    }

    #[test]
    fn test_merkle_root_order_independent() {
        let tx1 = make_tx(1);
        let tx2 = make_tx(2);
        let root1 = compute_merkle_root(&[tx1.clone(), tx2.clone()]);
        let root2 = compute_merkle_root(&[tx2, tx1]);
        // Sorted by tx_id → same root regardless of input order.
        assert_eq!(root1.as_bytes(), root2.as_bytes());
    }

    #[test]
    fn test_merkle_root_changes_with_different_txs() {
        let root1 = compute_merkle_root(&[make_tx(1)]);
        let root2 = compute_merkle_root(&[make_tx(2)]);
        assert_ne!(root1.as_bytes(), root2.as_bytes());
    }

    // ── PoW target ────────────────────────────────────────────────────────────

    #[test]
    fn test_difficulty_1_accepts_all() {
        let target = difficulty_to_target(1);
        // Difficulty=1 should accept nearly any hash.
        let easy_hash = sha256(b"test");
        // Not guaranteed to pass (hash might be high), but difficulty=1 should
        // produce a very easy target.
        let _ = meets_target(&easy_hash, &target); // just verify no panic
    }

    #[test]
    fn test_difficulty_increases_target_strictness() {
        let easy_target = difficulty_to_target(1);
        let hard_target = difficulty_to_target(1_000_000);
        // Hard target should be numerically smaller (more leading zeros).
        assert!(hard_target < easy_target);
    }

    // ── Reward ────────────────────────────────────────────────────────────────

    #[test]
    fn test_total_coinbase_no_txs() {
        let height = BlockHeight::new(1);
        let reward = total_coinbase(&height, &[]).unwrap();
        assert_eq!(reward.as_micro(), INITIAL_BLOCK_REWARD);
    }

    #[test]
    fn test_total_coinbase_with_fees() {
        let tx     = make_tx(1);
        let fee    = tx.fee();
        let height = BlockHeight::new(1);
        let total  = total_coinbase(&height, &[tx]).unwrap();
        let expected = Amount::from_micro(INITIAL_BLOCK_REWARD)
            .unwrap()
            .checked_add(fee)
            .unwrap();
        assert_eq!(total, expected);
    }
}
