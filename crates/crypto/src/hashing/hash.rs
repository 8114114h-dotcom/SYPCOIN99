// hashing/hash.rs — SHA-256 and Keccak-256 digest functions.
//
// Security decisions:
//   • Neither Sha256Hasher nor Keccak256Hasher is exported publicly.
//     Callers use the free functions sha256() / keccak256() in lib.rs.
//     This means the algorithm choice is an internal concern; downstream
//     modules cannot accidentally hard-code a hasher type.
//
//   • HashDigest wraps [u8; 32] in a newtype. Passing raw arrays between
//     modules erases intent — a HashDigest is unambiguously a digest, not
//     an arbitrary byte buffer, a key, or an address.

use sha2::{Digest as Sha2Digest, Sha256};
#[allow(unused_imports)]
use sha3::{Digest as _, Keccak256};
use serde::{Deserialize, Serialize};

/// A 32-byte cryptographic digest.
///
/// Produced by [`sha256`] or [`keccak256`]. Treated as opaque by callers;
/// the internal byte array is accessible only through `as_bytes()`.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct HashDigest(pub(crate) [u8; 32]);

impl std::hash::Hash for HashDigest {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}


impl HashDigest {
    /// Returns the raw digest bytes. Read-only; callers cannot mutate.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

// ─── Internal hasher implementations ─────────────────────────────────────────
// These structs are pub(crate) so only this crate can instantiate them.
// The Hasher trait is not exported, preventing downstream algorithm selection.

pub(crate) struct Sha256Hasher;
pub(crate) struct Keccak256Hasher;

pub(crate) trait Hasher {
    fn digest(input: &[u8]) -> HashDigest;
}

impl Hasher for Sha256Hasher {
    fn digest(input: &[u8]) -> HashDigest {
        // sha2::Sha256 implements the RustCrypto `Digest` trait.
        // One-shot hashing: no streaming state left in memory after this call.
        let result = Sha256::digest(input);
        // GenericArray<u8, U32> → [u8; 32]: safe because SHA-256 always
        // produces exactly 32 bytes.
        HashDigest(result.into())
    }
}

impl Hasher for Keccak256Hasher {
    fn digest(input: &[u8]) -> HashDigest {
        // sha3::Keccak256 is the *original* Keccak (pre-NIST padding), which
        // is what Ethereum uses. If NIST SHA-3 is ever needed, use sha3::Sha3_256.
        let result = Keccak256::digest(input);
        HashDigest(result.into())
    }
}

// ─── Public free functions (re-exported from lib.rs) ─────────────────────────

/// Compute SHA-256 of `input`. Default hasher for address derivation and
/// block header linking.
pub fn sha256(input: &[u8]) -> HashDigest {
    Sha256Hasher::digest(input)
}

/// Compute Keccak-256 of `input`. Available for EVM-compatible use cases.
pub fn keccak256(input: &[u8]) -> HashDigest {
    Keccak256Hasher::digest(input)
}
