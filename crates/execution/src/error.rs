// error.rs — Unified error type for the execution crate.

use thiserror::Error;
use state::StateError;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ExecutionError {
    /// A state mutation failed (insufficient balance, nonce mismatch, etc.).
    #[error("state error: {0}")]
    StateError(#[from] StateError),

    /// A transaction or block failed structural validation.
    #[error("validation error: {0}")]
    ValidationError(String),

    /// The block's height does not follow the current state height.
    #[error("block height mismatch: expected {expected}, got {got}")]
    BlockHeightMismatch { expected: u64, got: u64 },

    /// Attempted to execute a block with no transactions (allowed but noted).
    #[error("block has no transactions")]
    EmptyBlock,

    /// Block reward application failed.
    #[error("reward application failed: {0}")]
    RewardFailed(String),
}
