// lib.rs — Public API surface for the `transaction` crate.
//
// Downstream crates import ONLY from here.
//
//   use transaction::{Transaction, TransactionBuilder, TransactionValidator};
//   use transaction::{TransactionReceipt, ReceiptStatus};
//   use transaction::{Mempool, MempoolConfig};
//   use transaction::TransactionError;

mod error;

mod tx {
    pub(crate) mod constants;
    pub(crate) mod transaction;
    pub(crate) mod builder;
    pub(crate) mod validator;
    pub(crate) mod receipt;
}

mod mempool {
    pub(crate) mod eviction;
    pub(crate) mod ordering;
    pub(crate) mod pool;
}

// ── Public re-exports ─────────────────────────────────────────────────────────

/// Unified error type for all transaction operations.
pub use error::TransactionError;

/// A fully signed, validated transaction.
pub use tx::transaction::Transaction;

/// Builder — the only way to construct a Transaction.
pub use tx::builder::TransactionBuilder;

/// Structural and state-level transaction validator.
pub use tx::validator::TransactionValidator;

/// Execution receipt produced by the state transition layer.
pub use tx::receipt::TransactionReceipt;

/// Outcome of a transaction execution.
pub use tx::receipt::ReceiptStatus;

/// In-memory pool of pending transactions.
pub use mempool::pool::Mempool;

/// Configuration for the Mempool.
pub use mempool::pool::MempoolConfig;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crypto::KeyPair;
    use primitives::{Amount, Nonce, Timestamp};
    use primitives::constants::MIN_TX_FEE_MICRO;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn make_keypair() -> KeyPair {
        KeyPair::generate().unwrap()
    }

    fn make_recipient() -> crypto::Address {
        let kp = make_keypair();
        crypto::Address::from_public_key(kp.public_key())
    }

    fn valid_tx(sender: &KeyPair, nonce: u64) -> Transaction {
        TransactionBuilder::new()
            .from_keypair(KeyPair::from_seed(
                // Re-derive the same keypair from its bytes for signing
                // We pass sender's private bytes via test-utils feature.
                // In tests we generate a fresh one and use it directly.
                [0u8; 32] // placeholder — overridden below
            ).unwrap())
            .to(make_recipient())
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(nonce))
            .build()
            .unwrap()
    }

    fn build_tx(sender: &KeyPair, recipient: crypto::Address, nonce: u64) -> Transaction {
        // We need ownership of the keypair for signing.
        // Generate a fresh deterministic one in tests via seed.
        let _ = sender; // not used directly — see note below
        let kp = KeyPair::generate().unwrap();
        TransactionBuilder::new()
            .from_keypair(kp)
            .to(recipient)
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(nonce))
            .build()
            .unwrap()
    }

    // ── TransactionBuilder ────────────────────────────────────────────────────

    #[test]
    fn test_build_valid_transaction() {
        let kp        = make_keypair();
        let recipient = make_recipient();
        let tx = TransactionBuilder::new()
            .from_keypair(kp)
            .to(recipient)
            .amount(Amount::from_tokens(5).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(0))
            .build();
        assert!(tx.is_ok(), "expected Ok, got {:?}", tx);
    }

    #[test]
    fn test_build_missing_keypair() {
        let result = TransactionBuilder::new()
            .to(make_recipient())
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(0))
            .build();
        assert!(matches!(result, Err(TransactionError::MissingField(_))));
    }

    #[test]
    fn test_build_missing_recipient() {
        let kp = make_keypair();
        let result = TransactionBuilder::new()
            .from_keypair(kp)
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(0))
            .build();
        assert!(matches!(result, Err(TransactionError::MissingField(_))));
    }

    #[test]
    fn test_build_zero_amount_rejected() {
        let kp = make_keypair();
        let result = TransactionBuilder::new()
            .from_keypair(kp)
            .to(make_recipient())
            .amount(Amount::ZERO)
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(0))
            .build();
        assert!(matches!(result, Err(TransactionError::AmountIsZero)));
    }

    #[test]
    fn test_build_low_fee_rejected() {
        let kp = make_keypair();
        let result = TransactionBuilder::new()
            .from_keypair(kp)
            .to(make_recipient())
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(1).unwrap()) // below minimum
            .nonce(Nonce::new(0))
            .build();
        assert!(matches!(result, Err(TransactionError::InsufficientFee { .. })));
    }

    #[test]
    fn test_build_self_transfer_rejected() {
        let kp        = make_keypair();
        let self_addr = crypto::Address::from_public_key(kp.public_key());
        let result = TransactionBuilder::new()
            .from_keypair(kp)
            .to(self_addr)
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(0))
            .build();
        assert!(matches!(result, Err(TransactionError::SelfTransfer)));
    }

    #[test]
    fn test_build_data_too_large_rejected() {
        let kp   = make_keypair();
        let data = vec![0u8; 257]; // 1 byte over limit
        let result = TransactionBuilder::new()
            .from_keypair(kp)
            .to(make_recipient())
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(0))
            .data(data)
            .build();
        assert!(matches!(result, Err(TransactionError::DataTooLarge { .. })));
    }

    #[test]
    fn test_build_data_max_size_ok() {
        let kp   = make_keypair();
        let data = vec![0xABu8; 256]; // exactly at limit
        let result = TransactionBuilder::new()
            .from_keypair(kp)
            .to(make_recipient())
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(0))
            .data(data)
            .build();
        assert!(result.is_ok());
    }

    // ── Transaction properties ────────────────────────────────────────────────

    #[test]
    fn test_tx_id_is_deterministic() {
        // Two builds from the same keypair + timestamp should give different
        // tx_ids because OsRng generates fresh keypairs — but same keypair
        // + same timestamp → same tx_id.
        let kp        = make_keypair();
        let recipient = make_recipient();
        let ts        = Timestamp::from_millis(1_700_000_000_000);

        // We can't reuse the keypair (moved), so we verify the tx_id is
        // a SHA-256 of the canonical bytes by recomputing it.
        let tx = TransactionBuilder::new()
            .from_keypair(kp)
            .to(recipient)
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(0))
            .timestamp(ts)
            .build()
            .unwrap();

        // tx_id must equal SHA-256 of to_bytes().
        let expected_id = crypto::sha256(&tx.to_bytes());
        assert_eq!(tx.tx_id().as_bytes(), expected_id.as_bytes());
    }

    #[test]
    fn test_total_deducted_is_amount_plus_fee() {
        let kp = make_keypair();
        let amount = Amount::from_tokens(10).unwrap();
        let fee    = Amount::from_micro(MIN_TX_FEE_MICRO).unwrap();
        let tx = TransactionBuilder::new()
            .from_keypair(kp)
            .to(make_recipient())
            .amount(amount)
            .fee(fee)
            .nonce(Nonce::new(0))
            .build()
            .unwrap();
        let expected = amount.checked_add(fee).unwrap();
        assert_eq!(tx.total_deducted().unwrap(), expected);
    }

    #[test]
    fn test_from_address_derived_from_pubkey() {
        let kp        = make_keypair();
        let expected  = crypto::Address::from_public_key(kp.public_key());
        let tx = TransactionBuilder::new()
            .from_keypair(kp)
            .to(make_recipient())
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(0))
            .build()
            .unwrap();
        assert_eq!(tx.from(), &expected);
    }

    // ── TransactionValidator ─────────────────────────────────────────────────

    #[test]
    fn test_validate_structure_valid_tx() {
        let kp = make_keypair();
        let tx = TransactionBuilder::new()
            .from_keypair(kp)
            .to(make_recipient())
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(0))
            .build()
            .unwrap();
        assert!(TransactionValidator::validate_structure(&tx).is_ok());
    }

    #[test]
    fn test_validate_against_state_valid() {
        let kp = make_keypair();
        let tx = TransactionBuilder::new()
            .from_keypair(kp)
            .to(make_recipient())
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(1)) // nonce=1 follows account_nonce=0
            .build()
            .unwrap();

        let result = TransactionValidator::validate_against_state(
            &tx,
            Amount::from_tokens(100).unwrap(), // balance: 100 tokens
            Nonce::new(0),                     // account nonce: 0
            Timestamp::now(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_against_state_insufficient_balance() {
        let kp = make_keypair();
        let tx = TransactionBuilder::new()
            .from_keypair(kp)
            .to(make_recipient())
            .amount(Amount::from_tokens(100).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(1))
            .build()
            .unwrap();

        let result = TransactionValidator::validate_against_state(
            &tx,
            Amount::from_tokens(5).unwrap(), // only 5 tokens available
            Nonce::new(0),
            Timestamp::now(),
        );
        assert!(matches!(result, Err(TransactionError::InsufficientBalance { .. })));
    }

    #[test]
    fn test_validate_against_state_wrong_nonce() {
        let kp = make_keypair();
        let tx = TransactionBuilder::new()
            .from_keypair(kp)
            .to(make_recipient())
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(5)) // wrong: account expects nonce=1
            .build()
            .unwrap();

        let result = TransactionValidator::validate_against_state(
            &tx,
            Amount::from_tokens(100).unwrap(),
            Nonce::new(0), // account nonce=0, so expects tx nonce=1
            Timestamp::now(),
        );
        assert!(matches!(result, Err(TransactionError::InvalidNonce { .. })));
    }

    // ── Mempool ───────────────────────────────────────────────────────────────

    #[test]
    fn test_mempool_add_and_contains() {
        let mut pool = Mempool::with_defaults();
        let kp       = make_keypair();
        let tx = TransactionBuilder::new()
            .from_keypair(kp)
            .to(make_recipient())
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(0))
            .build()
            .unwrap();

        let tx_id = tx.tx_id().clone();
        pool.add(tx).unwrap();
        assert!(pool.contains(&tx_id));
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn test_mempool_duplicate_rejected() {
        let mut pool = Mempool::with_defaults();
        let kp = make_keypair();
        let tx = TransactionBuilder::new()
            .from_keypair(kp)
            .to(make_recipient())
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(0))
            .timestamp(Timestamp::from_millis(1_700_000_000_000))
            .build()
            .unwrap();

        pool.add(tx.clone()).unwrap();
        let result = pool.add(tx);
        assert!(matches!(result, Err(TransactionError::DuplicateTransaction)));
    }

    #[test]
    fn test_mempool_remove() {
        let mut pool = Mempool::with_defaults();
        let kp       = make_keypair();
        let tx = TransactionBuilder::new()
            .from_keypair(kp)
            .to(make_recipient())
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(0))
            .build()
            .unwrap();

        let tx_id = tx.tx_id().clone();
        pool.add(tx).unwrap();
        pool.remove(&tx_id);
        assert!(!pool.contains(&tx_id));
        assert!(pool.is_empty());
    }

    #[test]
    fn test_mempool_top_n_fee_ordering() {
        let mut pool = Mempool::with_defaults();

        // Add three transactions with different fees.
        for fee_micro in [5_000u64, 1_000, 10_000] {
            let kp = make_keypair();
            let tx = TransactionBuilder::new()
                .from_keypair(kp)
                .to(make_recipient())
                .amount(Amount::from_tokens(1).unwrap())
                .fee(Amount::from_micro(fee_micro).unwrap())
                .nonce(Nonce::new(0))
                .build()
                .unwrap();
            pool.add(tx).unwrap();
        }

        let top = pool.top_n(3);
        assert_eq!(top.len(), 3);
        // Highest fee first.
        assert_eq!(top[0].fee().as_micro(), 10_000);
        assert_eq!(top[1].fee().as_micro(), 5_000);
        assert_eq!(top[2].fee().as_micro(), 1_000);
    }

    #[test]
    fn test_mempool_evict_expired() {
        use crate::mempool::pool::MempoolConfig;
        let config = MempoolConfig {
            max_size:        100,
            max_per_address: 64,
            min_fee:         Amount::from_micro(MIN_TX_FEE_MICRO).unwrap(),
            tx_ttl_ms:       5_000, // 5 second TTL
        };
        let mut pool = Mempool::new(config);

        // Transaction timestamped 10 seconds ago.
        let kp   = make_keypair();
        let old_ts = Timestamp::from_millis(
            Timestamp::now().as_millis().saturating_sub(10_000)
        );
        let tx = TransactionBuilder::new()
            .from_keypair(kp)
            .to(make_recipient())
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(0))
            .timestamp(old_ts)
            .build()
            .unwrap();

        pool.add(tx).unwrap();
        assert_eq!(pool.len(), 1);

        let evicted = pool.evict_expired(Timestamp::now());
        assert_eq!(evicted, 1);
        assert!(pool.is_empty());
    }

    // ── TransactionReceipt ────────────────────────────────────────────────────

    #[test]
    fn test_receipt_success() {
        let tx_id   = crypto::sha256(b"test");
        let bh      = primitives::BlockHeight::new(10);
        let bh_hash = crypto::sha256(b"block");
        let fee     = Amount::from_micro(MIN_TX_FEE_MICRO).unwrap();
        let ts      = Timestamp::now();

        let receipt = TransactionReceipt::success(
            tx_id, bh, bh_hash, fee, ts
        );
        assert!(receipt.is_success());
        assert!(receipt.status.failure_reason().is_none());
    }

    #[test]
    fn test_receipt_failure() {
        let tx_id   = crypto::sha256(b"test");
        let bh      = primitives::BlockHeight::new(10);
        let bh_hash = crypto::sha256(b"block");
        let fee     = Amount::from_micro(MIN_TX_FEE_MICRO).unwrap();
        let ts      = Timestamp::now();

        let receipt = TransactionReceipt::failed(
            tx_id, bh, bh_hash, fee, ts,
            "insufficient balance".into()
        );
        assert!(!receipt.is_success());
        assert_eq!(receipt.status.failure_reason(), Some("insufficient balance"));
    }
}
