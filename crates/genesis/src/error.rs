// error.rs — Unified error type for the genesis crate.

use thiserror::Error;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum GenesisError {
    #[error("chain_id mismatch: config has {got}, binary expects {expected}")]
    InvalidChainId { expected: u64, got: u64 },

    #[error("initial_difficulty must be ≥ 1")]
    InvalidDifficulty,

    #[error("invalid genesis account '{address}': {reason}")]
    InvalidAccount { address: String, reason: String },

    #[error("genesis supply {total} exceeds MAX_SUPPLY {max} micro-tokens")]
    SupplyExceeded { total: u64, max: u64 },

    #[error("genesis config parse error: {0}")]
    ParseError(String),

    #[error("genesis state error: {0}")]
    StateError(String),

    #[error("genesis block build error: {0}")]
    BlockBuildError(String),

    #[error("no genesis accounts defined")]
    NoAccounts,
    #[error("genesis build failed: {0}")]
    BuildFailed(String),
}
