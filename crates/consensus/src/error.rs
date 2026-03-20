// error.rs — Unified error type for the consensus crate.

use thiserror::Error;

#[non_exhaustive]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConsensusError {
    /// Block hash does not meet the PoW target.
    #[error("invalid proof-of-work")]
    InvalidPoW,

    /// Block difficulty does not match the expected value.
    #[error("invalid difficulty: expected {expected}, got {got}")]
    InvalidDifficulty { expected: u64, got: u64 },

    /// Miner claimed more reward than allowed.
    #[error("invalid block reward")]
    InvalidBlockReward,

    /// Block references a parent we have not seen.
    #[error("orphan block — parent not found")]
    OrphanBlock,

    /// Attempted reorg deeper than MAX_REORG_DEPTH.
    #[error("fork too deep: depth {depth}")]
    ForkTooDeep { depth: u64 },

    /// Reorg exceeds MAX_REORG_DEPTH — 51% attack guard.
    #[error("reorg too deep: depth {depth} > max {max} — possible 51% attack")]
    ReorgTooDeep { depth: u64, max: u64 },

    /// Requested chain not found.
    #[error("chain not found")]
    ChainNotFound,

    /// Block is already in the chain.
    #[error("block already known")]
    BlockAlreadyKnown,

    /// Genesis block is invalid or has already been set.
    #[error("invalid genesis block")]
    InvalidGenesisBlock,

    /// Mining loop exhausted all nonces without finding a solution.
    #[error("mining failed: no valid nonce found in range")]
    MiningFailed,

    /// Block validation failed (wraps block::BlockError message).
    #[error("block validation failed: {0}")]
    BlockValidation(String),

    /// Difficulty adjustment produced an out-of-range value.
    #[error("difficulty adjustment overflow")]
    DifficultyOverflow,
}
