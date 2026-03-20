// error.rs — Unified error type for the state crate.

use thiserror::Error;
use crypto::Address;
use primitives::Amount;

#[non_exhaustive]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum StateError {
    /// Tried to read/debit an account that does not exist.
    #[error("account not found: {0}")]
    AccountNotFound(String),

    /// Debit would make balance go negative.
    #[error("insufficient balance: available {available}, required {required}")]
    InsufficientBalance { available: Amount, required: Amount },
    /// Balance would overflow MAX_SUPPLY on addition.
    #[error("balance overflow — would exceed maximum supply")]
    BalanceOverflow,

    /// Nonce counter overflowed u64 (practically impossible).
    #[error("nonce overflow for account")]
    NonceOverflow,

    /// Adding to total_supply would overflow u64.
    #[error("supply arithmetic overflow")]
    SupplyOverflow,

    /// Adding to total_supply would exceed MAX_SUPPLY_MICRO.
    #[error("supply would exceed maximum: attempting to add {0} micro-tokens")]
    SupplyExceedsMax(u64),

    /// A state transition was attempted with invalid parameters.
    #[error("invalid state transition: {0}")]
    InvalidTransition(String),

    /// No snapshot found at the requested height.
    #[error("no snapshot at height {0}")]
    SnapshotNotFound(u64),

    /// Rollback failed because journal was empty or inconsistent.
    #[error("rollback failed: {0}")]
    RollbackFailed(String),
}

impl StateError {
    /// Convenience constructor for AccountNotFound.
    pub fn account_not_found(addr: &Address) -> Self {
        StateError::AccountNotFound(addr.to_checksum_hex())
    }
}
