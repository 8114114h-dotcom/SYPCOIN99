// error.rs — Security error types.

use thiserror::Error;

#[non_exhaustive]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SecurityError {
    #[error("spam detected from {address}: {reason}")]
    SpamDetected { address: String, reason: String },

    #[error("replay attack: tx_id {tx_id} already seen")]
    ReplayDetected { tx_id: String },

    #[error("blacklisted address: {address}")]
    BlacklistedAddress { address: String },

    #[error("transaction too large: max {max} bytes, got {got}")]
    TxTooLarge { max: usize, got: usize },

    #[error("fee too low: minimum {min} micro-tokens, got {got}")]
    FeeTooLow { min: u64, got: u64 },

    #[error("block spam: address {address} has {count} txs, max is {max}")]
    BlockSpamDetected { address: String, count: u32, max: u32 },
}
