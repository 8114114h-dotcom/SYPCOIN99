// error.rs — Unified error enum for the entire crypto crate.
//
// #[non_exhaustive] is intentional: downstream crates matching on CryptoError
// must include a wildcard arm (`_ => ...`). This lets us add new variants in
// minor versions without breaking callers — an explicit semver stability promise.

use thiserror::Error;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("invalid public key bytes")]
    InvalidPublicKey,

    #[error("invalid signature bytes")]
    InvalidSignature,

    #[error("invalid address bytes")]
    InvalidAddress,

    #[error("address checksum mismatch")]
    InvalidChecksum,

    #[error("signing operation failed")]
    SigningFailed,

    #[error("signature verification failed")]
    VerificationFailed,
}
