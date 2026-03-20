// error.rs — Unified error type for the block crate.

use thiserror::Error;

#[non_exhaustive]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BlockError {
    #[error("unsupported block version: {0}")]
    InvalidVersion(u8),

    #[error("invalid block height: expected {expected}, got {got}")]
    InvalidHeight { expected: u64, got: u64 },

    #[error("invalid parent hash")]
    InvalidParentHash,

    #[error("merkle root mismatch: computed root does not match header")]
    InvalidMerkleRoot,

    #[error("state root is missing or zero")]
    InvalidStateRoot,

    #[error("block timestamp is invalid: {0}")]
    InvalidTimestamp(String),

    #[error("difficulty is zero")]
    InvalidDifficulty,

    #[error("insufficient proof-of-work: hash {hash} >= target {target}")]
    InsufficientPoW { hash: String, target: String },

    #[error("too many transactions: max {max}, got {got}")]
    TooManyTransactions { max: u32, got: u32 },

    #[error("block too large: max {max} bytes, got {got} bytes")]
    BlockTooLarge { max: usize, got: usize },

    #[error("duplicate transaction in block")]
    DuplicateTransaction,

    #[error("missing required field: {0}")]
    MissingField(String),

    #[error("genesis block already exists")]
    GenesisAlreadyExists,

    #[error("block body merkle root does not match transaction list")]
    MerkleRootMismatch,

    #[error("transaction count in header ({header}) does not match body ({body})")]
    TxCountMismatch { header: u32, body: u32 },
}
