// error.rs — Unified error type for the networking crate.

use thiserror::Error;

/// A unique peer identifier (SHA-256 of socket address string).
pub type PeerId = [u8; 32];

#[non_exhaustive]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum NetworkError {
    #[error("peer not found")]
    PeerNotFound,

    #[error("peer is banned")]
    PeerBanned,

    #[error("rate limit exceeded for peer")]
    RateLimitExceeded,

    #[error("handshake failed: {0}")]
    HandshakeFailed(String),

    #[error("incompatible protocol version: ours={ours}, theirs={theirs}")]
    IncompatibleVersion { ours: u32, theirs: u32 },

    #[error("wrong chain id: expected {expected}, got {got}")]
    WrongChainId { expected: u64, got: u64 },

    #[error("message too large: max {max} bytes, got {got} bytes")]
    MessageTooLarge { max: usize, got: usize },

    #[error("codec error: {0}")]
    CodecError(String),

    #[error("maximum peer count reached")]
    MaxPeersReached,

    #[error("peer already connected")]
    PeerAlreadyConnected,
}
