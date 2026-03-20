// error.rs — Unified error type for the transaction crate.
//
// #[non_exhaustive] ensures downstream match arms include a wildcard,
// allowing new variants to be added without breaking callers.

use thiserror::Error;
use primitives::{Amount, Nonce};

#[non_exhaustive]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TransactionError {
    /// Ed25519 signature verification failed.
    #[error("invalid transaction signature")]
    InvalidSignature,

    /// Transaction targets a different chain.
    #[error("wrong chain id: expected {expected}, got {got}")]
    InvalidChainId { expected: u64, got: u64 },

    /// Unsupported transaction version byte.
    #[error("unsupported transaction version: {0}")]
    InvalidVersion(u8),

    /// Sender does not have enough balance to cover amount + fee.
    #[error("insufficient balance: available {available}, required {required}")]
    InsufficientBalance { available: Amount, required: Amount },

    /// Fee is below the minimum threshold.
    #[error("insufficient fee: minimum {minimum}, provided {provided}")]
    InsufficientFee { minimum: Amount, provided: Amount },

    /// Transaction nonce does not match the sender's current account nonce.
    #[error("invalid nonce: expected {expected}, got {got}")]
    InvalidNonce { expected: Nonce, got: Nonce },

    /// Sender and recipient are the same address.
    #[error("self-transfer is not allowed")]
    SelfTransfer,

    /// Transfer amount is zero.
    #[error("transfer amount must be greater than zero")]
    AmountIsZero,

    /// Optional data payload exceeds maximum allowed size.
    #[error("data too large: max {max} bytes, got {got}")]
    DataTooLarge { max: usize, got: usize },

    /// Serialized transaction exceeds maximum block transaction size.
    #[error("transaction too large: max {max} bytes, got {got}")]
    TransactionTooLarge { max: usize, got: usize },

    /// Timestamp is too far in the future.
    #[error("transaction timestamp is too far in the future")]
    TimestampInFuture,

    /// Transaction has been in the mempool too long without confirmation.
    #[error("transaction has expired")]
    TransactionExpired,

    /// Mempool has reached its maximum capacity.
    #[error("mempool is full")]
    MempoolFull,

    /// Sender already has the maximum number of pending transactions.
    #[error("address mempool limit reached")]
    MempoolAddressLimitReached,

    /// A transaction with the same tx_id already exists in the mempool.
    #[error("duplicate transaction")]
    DuplicateTransaction,

    /// A required field was not set on the builder.
    #[error("missing required field: {0}")]
    MissingField(String),

    /// Amount arithmetic overflow during transaction construction.
    #[error("amount arithmetic overflow")]
    AmountOverflow,

    /// The signing operation failed (OS entropy unavailable).
    #[error("signing failed")]
    SigningFailed,
}
