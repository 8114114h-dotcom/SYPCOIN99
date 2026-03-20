// error.rs — Unified error type for the primitives crate.
//
// #[non_exhaustive] is intentional: downstream crates matching on
// PrimitivesError must include a wildcard arm. This lets us add new
// variants in minor versions without breaking callers.

use thiserror::Error;

#[non_exhaustive]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PrimitivesError {
    /// Integer overflow during Amount arithmetic.
    #[error("amount arithmetic overflow")]
    AmountOverflow,

    /// The resulting amount would exceed MAX_SUPPLY.
    #[error("amount exceeds maximum supply ({0} micro-tokens)")]
    AmountExceedsMaxSupply(u64),

    /// A whole-token value could not be converted (e.g. too large).
    #[error("invalid token amount: {0}")]
    InvalidTokenAmount(String),

    /// A timestamp is implausibly far in the future.
    #[error("timestamp is too far in the future")]
    TimestampInFuture,

    /// A nonce value overflowed u64 (practically impossible but handled).
    #[error("nonce overflow")]
    NonceOverflow,
}
