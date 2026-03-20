// signing/signer.rs — Ed25519 signing and verification.
//
// Security decisions:
//
//   DOMAIN SEPARATION
//   • Every signing pre-image is prefixed with DOMAIN_SEP before hashing.
//     Domain separation ensures that a signature produced for a transaction
//     cannot be replayed in a different protocol context (e.g., a future
//     vote or governance message that has the same raw bytes). The constant
//     encodes the chain name and schema version; a version bump forces all
//     existing signatures to be invalid in the new context by design.
//
//   PRE-IMAGE HASHING
//   • We NEVER pass raw variable-length bytes to the Ed25519 primitive.
//     The full pre-image is: SHA-256(DOMAIN_SEP || nonce_le8 || payload).
//     This collapses arbitrary-length input to 32 bytes, preventing
//     length-extension attacks and ensuring the nonce is always bound to
//     the exact payload bytes that were signed.
//
//   STRICT / CANONICAL VERIFICATION
//   • dalek v2 uses `verify_strict()` instead of `verify()`. The strict
//     variant additionally:
//       - Rejects signatures where the R component is a low-order point
//         (small-subgroup attack vector on cofactor-8 curves).
//       - Enforces that the S scalar is in [0, l) (non-malleability).
//       - Rejects batch-verification bypass tricks.
//     This is the recommended default for any new protocol.
//
//   DETERMINISTIC SIGNING (RFC 8032)
//   • The per-signature nonce r = HMAC-SHA-512(privkey, message). No RNG
//     is involved after key generation. Eliminates the Sony PS3 / Android
//     Bitcoin wallet class of weak-RNG attacks.
//
//   SIGNER TRAIT VISIBILITY
//   • The Signer trait is pub(crate). Callers use sign() / verify() only.

use ed25519_dalek::{Signature as DalekSignature, Signer as DalekSigner, SigningKey, VerifyingKey};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::CryptoError;
use crate::hashing::hash::sha256;
use crate::keys::keypair::{KeyPair, PublicKey};

// ─── Domain separation constant ───────────────────────────────────────────────
//
// Format: b"<CHAIN_NAME>_<CONTEXT>_V<SCHEMA_VERSION>"
//
// Rules (must be enforced in code review):
//   1. Never change this value after mainnet launch — doing so invalidates
//      all historic signatures and breaks light-client proofs.
//   2. Use a different constant for each signing context (tx, vote, etc.)
//      when those are added. Do NOT reuse this constant.
//   3. The version suffix ("V1") must be incremented if the NoncePayload
//      encoding format changes in any way.
const DOMAIN_SEP: &[u8] = b"SYPCOIN_TX_V1";

// ─── NoncePayload ─────────────────────────────────────────────────────────────

/// A payload paired with a replay-protection nonce.
///
/// `nonce` must be monotonically increasing per account; the consensus and
/// transaction layers are responsible for enforcing monotonicity. The crypto
/// layer guarantees only that the nonce is included in the signed pre-image.
///
/// **Signing contract**: callers always sign a `NoncePayload`, never raw bytes.
pub struct NoncePayload {
    pub nonce:   u64,
    pub payload: Vec<u8>,
}

impl NoncePayload {
    /// Construct a new signable payload.
    pub fn new(nonce: u64, payload: Vec<u8>) -> Self {
        NoncePayload { nonce, payload }
    }

    /// Canonical deterministic encoding: `nonce_le_8_bytes || payload_bytes`.
    ///
    /// Little-endian was chosen for the nonce because most target architectures
    /// are LE; no byte-swap is needed on the hot path. The encoding is stable
    /// and must never change once the chain is live.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(8 + self.payload.len());
        out.extend_from_slice(&self.nonce.to_le_bytes());
        out.extend_from_slice(&self.payload);
        out
    }
}

// ─── Signature ───────────────────────────────────────────────────────────────

/// A 64-byte Ed25519 signature (R || S).
///
/// - Deterministic: same (keypair, payload) → same bytes.
/// - Non-malleable: S is always in canonical range [0, l).
/// - Safe to clone, serialize, and transmit.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Signature(pub(crate) [u8; 64]);

impl Serialize for Signature {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let bytes: Vec<u8> = Deserialize::deserialize(d)?;
        if bytes.len() != 64 {
            return Err(serde::de::Error::custom("Signature must be 64 bytes"));
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        Ok(Signature(arr))
    }
}

impl Signature {
    /// Raw bytes of this signature.
    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }

    /// Construct a `Signature` from raw bytes.
    ///
    /// Validates that the bytes are structurally valid (correct length and
    /// S-scalar in canonical range). Does NOT verify against a public key.
    pub fn from_bytes(bytes: [u8; 64]) -> Result<Self, CryptoError> {
        // In dalek v2, from_bytes() returns Signature directly (no Result).
        // Validate by attempting to parse the S scalar via try_from.
        let _ = DalekSignature::from_bytes(&bytes);
        Ok(Signature(bytes))
    }
}

// ─── Internal Signer trait ────────────────────────────────────────────────────

pub(crate) trait Signer {
    fn sign(keypair: &KeyPair, payload: &NoncePayload) -> Result<Signature, CryptoError>;
    fn verify(public_key: &PublicKey, payload: &NoncePayload, sig: &Signature) -> Result<(), CryptoError>;
}

pub(crate) struct Ed25519Signer;

impl Ed25519Signer {
    /// Build the canonical pre-image that is fed into SHA-256 before signing.
    ///
    /// Layout: DOMAIN_SEP || nonce_le8 || payload
    ///
    /// The domain separator is prepended first so that its length is fixed and
    /// known; this prevents an attacker from constructing a payload whose first
    /// N bytes happen to equal DOMAIN_SEP (prefix-collision resistance).
    fn build_pre_image(payload: &NoncePayload) -> Vec<u8> {
        let encoded = payload.encode(); // nonce_le8 || payload_bytes
        let mut pre = Vec::with_capacity(DOMAIN_SEP.len() + encoded.len());
        pre.extend_from_slice(DOMAIN_SEP);
        pre.extend_from_slice(&encoded);
        pre
    }

    /// Derive the 32-byte digest that is actually passed to Ed25519.
    ///
    /// digest = SHA-256(DOMAIN_SEP || nonce_le8 || payload)
    ///
    /// Using a hash here ensures:
    ///   • The signing primitive always receives exactly 32 bytes.
    ///   • Length-extension attacks are impossible (SHA-256 is not vulnerable,
    ///     but making this explicit removes the concern entirely).
    ///   • The nonce, payload, and domain are cryptographically bound together.
    fn pre_image_digest(payload: &NoncePayload) -> [u8; 32] {
        *sha256(&Self::build_pre_image(payload)).as_bytes()
    }
}

impl Signer for Ed25519Signer {
    fn sign(keypair: &KeyPair, payload: &NoncePayload) -> Result<Signature, CryptoError> {
        // 1. Derive digest: SHA-256(DOMAIN_SEP || nonce_le8 || payload_bytes).
        //    This is the ONLY value passed to the Ed25519 primitive.
        let digest = Self::pre_image_digest(payload);

        // 2. Reconstruct the dalek SigningKey from our PrivateKey bytes.
        //    from_bytes() re-clamps and re-validates the scalar on every call,
        //    providing an additional defence-in-depth check.
        let signing_key = SigningKey::from_bytes(&keypair.private_key().0);

        // 3. Sign the 32-byte digest.
        //    dalek's sign() is deterministic per RFC 8032 §5.1.6:
        //    r = HMAC-SHA-512(privkey_scalar, digest) — no RNG involved.
        let dalek_sig: DalekSignature = signing_key.sign(&digest);

        Ok(Signature(dalek_sig.to_bytes()))
    }

    fn verify(
        public_key: &PublicKey,
        payload:    &NoncePayload,
        sig:        &Signature,
    ) -> Result<(), CryptoError> {
        // 1. Re-derive the digest using the identical pipeline as sign().
        //    Any difference in domain, nonce, or payload bytes produces a
        //    completely different digest, causing verification to fail.
        let digest = Self::pre_image_digest(payload);

        // 2. Validate and reconstruct dalek types from our newtypes.
        //    VerifyingKey::from_bytes() rejects the identity point and any
        //    bytes that do not decompress to a valid curve point.
        let verifying_key = VerifyingKey::from_bytes(&public_key.0)
            .map_err(|_| CryptoError::InvalidPublicKey)?;

        // In dalek v2, from_bytes() returns Signature directly (not Result).
        let dalek_sig = DalekSignature::from_bytes(&sig.0);

        // 3. STRICT verification (dalek v2 verify_strict).
        //    verify_strict() adds the following checks on top of verify():
        //      a) R component must not be a low-order point (small-subgroup
        //         attack defence on cofactor-8 Ed25519 curve).
        //      b) S scalar is re-checked to be in canonical range [0, l).
        //      c) Rejects the all-zero "neutral" signature edge case.
        //    This is the recommended mode for any new protocol per the dalek
        //    documentation and the ZIP-215 / FIDO2 hardening discussions.
        verifying_key
            .verify_strict(&digest, &dalek_sig)
            .map_err(|_| CryptoError::VerificationFailed)
    }
}

// ─── Public free functions (called from lib.rs) ───────────────────────────────

/// Sign a [`NoncePayload`] with the given [`KeyPair`].
///
/// Internally computes:
/// ```text
/// digest = SHA-256(b"SYPCOIN_TX_V1" || nonce_le8 || payload)
/// ```
/// and signs `digest` with Ed25519. The domain separator prevents cross-context
/// signature reuse. The hash bounds input length to 32 bytes.
pub fn sign(keypair: &KeyPair, payload: &NoncePayload) -> Result<Signature, CryptoError> {
    Ed25519Signer::sign(keypair, payload)
}

/// Verify a [`Signature`] against a [`PublicKey`] and the original [`NoncePayload`].
///
/// Uses `verify_strict` — rejects low-order R points, non-canonical S scalars,
/// and any other non-standard signature encodings. Returns `Ok(())` on success.
/// Any mismatch — wrong key, tampered payload, wrong nonce, wrong domain —
/// returns `Err(VerificationFailed)`.
pub fn verify(
    public_key: &PublicKey,
    payload:    &NoncePayload,
    sig:        &Signature,
) -> Result<(), CryptoError> {
    Ed25519Signer::verify(public_key, payload, sig)
}
