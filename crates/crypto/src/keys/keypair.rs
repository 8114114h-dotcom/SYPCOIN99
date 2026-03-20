// keys/keypair.rs — KeyPair and PublicKey.
//
// Security decisions:
//
//   OsRng EXCLUSIVELY
//   • `rand::rngs::OsRng` is the only RNG used in production paths. It is a
//     zero-size type that calls the OS entropy source on every invocation
//     (getrandom(2) on Linux, BCryptGenRandom on Windows, getentropy(2) on
//     macOS). It cannot be seeded, configured, or accidentally replaced with
//     a weaker generator. We explicitly annotate the variable type as `OsRng`
//     (not just `_`) so that a future refactor cannot silently swap it.
//
//   INTERMEDIATE SCALAR ZEROIZED
//   • SigningKey::to_bytes() copies the 32-byte seed scalar onto our stack.
//     We wrap it in a zeroize::Zeroizing guard so the stack memory is wiped
//     even if From::from_dalek() returns early via `?` or panics.
//
//   from_seed() COMPILE-GATED
//   • Deterministic key generation is gated behind #[cfg(any(test,
//     feature = "test-utils"))]. It cannot appear in production binaries
//     unless the caller explicitly enables the feature flag.
//
//   DALEK TYPE ENCAPSULATION
//   • ed25519_dalek types never appear in our public API. PublicKey wraps
//     the CompressedEdwardsY bytes in our newtype. If the signing library
//     changes, only this file and signer.rs need updates.

use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::error::CryptoError;
use super::private_key::PrivateKey;

/// A 32-byte compressed Ed25519 public key.
///
/// Safe to clone, store, log, serialize, and transmit.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct PublicKey(pub(crate) [u8; 32]);

impl PublicKey {
    /// Raw byte representation of this public key.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Construct a `PublicKey` from raw bytes.
    ///
    /// Validates that the bytes represent a valid compressed Edwards point.
    /// Returns `Err(InvalidPublicKey)` for any invalid input, including the
    /// identity point and points not on the curve.
    pub fn from_bytes(bytes: [u8; 32]) -> Result<Self, CryptoError> {
        ed25519_dalek::VerifyingKey::from_bytes(&bytes)
            .map(|_| PublicKey(bytes))
            .map_err(|_| CryptoError::InvalidPublicKey)
    }
}

/// An Ed25519 keypair.
///
/// The only way to obtain access to signing capability is through this struct.
/// The private key material is never accessible outside the crate.
pub struct KeyPair {
    pub(crate) public_key:  PublicKey,
    pub(crate) private_key: PrivateKey,
}

impl KeyPair {
    /// Generate a fresh keypair using OS entropy.
    ///
    /// Uses `OsRng` exclusively — type-annotated to prevent accidental
    /// substitution. Calls `getrandom` / `BCryptGenRandom` under the hood.
    /// The OS RNG cannot be seeded from userspace.
    pub fn generate() -> Result<Self, CryptoError> {
        // Explicit type annotation: OsRng — not `_`, not thread_rng().
        let mut rng: OsRng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        Self::from_dalek(signing_key)
    }

    /// Deterministic keypair from a 32-byte seed.
    ///
    /// **DANGER**: Only available under `test-utils` feature or `#[cfg(test)]`.
    /// A weak or reused seed completely compromises all signatures. Must never
    /// be called in production code paths.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn from_seed(seed: [u8; 32]) -> Result<Self, CryptoError> {
        let signing_key = SigningKey::from_bytes(&seed);
        Self::from_dalek(signing_key)
    }

    /// The public half of this keypair. Safe to share freely.
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }

    /// Borrow the private key scalar. `pub(crate)` — only signer.rs uses this.
    pub(crate) fn private_key(&self) -> &PrivateKey {
        &self.private_key
    }

    // ─── Internal constructor ─────────────────────────────────────────────────

    fn from_dalek(signing_key: SigningKey) -> Result<Self, CryptoError> {
        // Wrap the extracted scalar in Zeroizing so the stack bytes are wiped
        // when `private_bytes` goes out of scope — even on an early return or
        // panic unwind. Without this, the 32-byte seed scalar would linger on
        // the stack until overwritten by the next function call.
        let private_bytes: Zeroizing<[u8; 32]> = Zeroizing::new(signing_key.to_bytes());

        // The verifying key is derived deterministically by dalek's scalar
        // multiplication; we extract only the compressed point bytes.
        let public_bytes: [u8; 32] = signing_key.verifying_key().to_bytes();

        Ok(KeyPair {
            public_key:  PublicKey(public_bytes),
            // PrivateKey copies the bytes out of the Zeroizing wrapper.
            // The wrapper is dropped here, zeroing the stack copy.
            // PrivateKey itself zeroizes its own copy on drop (ZeroizeOnDrop).
            private_key: PrivateKey(*private_bytes),
        })
    }
}
