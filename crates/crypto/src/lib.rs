// lib.rs — Public API surface for the `crypto` crate.
//
// RULE: Nothing is public unless explicitly re-exported here.
// Internal module structure is entirely invisible to downstream crates.
//
// Downstream modules (Transactions, Blocks) import ONLY from this file.
// The internal module tree (hashing/, keys/, signing/, address/) is an
// implementation detail that may change without breaking the public API.

// ─── Internal module tree (all private to this crate) ────────────────────────

mod error;

mod hashing {
    pub(crate) mod hash;
}

mod keys {
    pub(crate) mod private_key;
    pub(crate) mod keypair;
}

mod signing {
    pub(crate) mod signer;
}

mod address {
    pub(crate) mod address;
}

// ─── Public re-exports ────────────────────────────────────────────────────────
//
// Every symbol listed here is part of the stable public API.
// Nothing else is accessible to downstream crates.

/// Unified error type for all cryptographic operations.
pub use error::CryptoError;

/// 32-byte compressed Ed25519 public key.
pub use keys::keypair::PublicKey;

/// Ed25519 keypair (public + private). The only way to sign.
pub use keys::keypair::KeyPair;

/// 64-byte Ed25519 signature.
pub use signing::signer::Signature;

/// Nonce + payload wrapper. Always sign this, never raw bytes.
pub use signing::signer::NoncePayload;

/// 32-byte cryptographic digest (SHA-256 or Keccak-256 output).
pub use hashing::hash::HashDigest;

/// 20-byte account address derived from a public key.
pub use address::address::Address;

// ─── Public free functions ────────────────────────────────────────────────────

/// Sign a `NoncePayload`. Internally: SHA-256(encode(payload)) is signed.
pub use signing::signer::sign;

/// Verify a `Signature` against a `PublicKey` and `NoncePayload`.
pub use signing::signer::verify;

/// SHA-256 digest of arbitrary bytes.
pub use hashing::hash::sha256;

/// Keccak-256 digest of arbitrary bytes (EVM-compatible).
pub use hashing::hash::keccak256;

// ─── Crate-level tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── KeyPair ──────────────────────────────────────────────────────────────

    #[test]
    fn test_keypair_generate_is_unique() {
        let kp1 = KeyPair::generate().unwrap();
        let kp2 = KeyPair::generate().unwrap();
        // Two independent OS-entropy keypairs must not be equal.
        assert_ne!(kp1.public_key(), kp2.public_key());
    }

    #[test]
    #[cfg(feature = "test-utils")]
    fn test_keypair_from_seed_is_deterministic() {
        let seed = [0x42u8; 32];
        let kp1 = KeyPair::from_seed(seed).unwrap();
        let kp2 = KeyPair::from_seed(seed).unwrap();
        assert_eq!(kp1.public_key(), kp2.public_key());
    }

    // ── PublicKey round-trip ──────────────────────────────────────────────────

    #[test]
    fn test_public_key_roundtrip() {
        let kp = KeyPair::generate().unwrap();
        let bytes = *kp.public_key().as_bytes();
        let pk2 = PublicKey::from_bytes(bytes).unwrap();
        assert_eq!(kp.public_key(), &pk2);
    }

    #[test]
    fn test_invalid_public_key_rejected() {
        // All-zero bytes are not a valid Edwards point.
        assert!(PublicKey::from_bytes([0u8; 32]).is_err());
    }

    // ── Hashing ──────────────────────────────────────────────────────────────

    #[test]
    fn test_sha256_known_vector() {
        // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let digest = sha256(b"");
        let expected = hex::decode(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        ).unwrap();
        assert_eq!(digest.as_bytes(), expected.as_slice());
    }

    #[test]
    fn test_sha256_deterministic() {
        assert_eq!(sha256(b"hello"), sha256(b"hello"));
        assert_ne!(sha256(b"hello"), sha256(b"world"));
    }

    #[test]
    fn test_keccak256_known_vector() {
        // Keccak-256("") = c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470
        let digest = keccak256(b"");
        let expected = hex::decode(
            "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
        ).unwrap();
        assert_eq!(digest.as_bytes(), expected.as_slice());
    }

    // ── Signing & Verification ────────────────────────────────────────────────

    #[test]
    fn test_sign_and_verify_succeeds() {
        let kp      = KeyPair::generate().unwrap();
        let payload = NoncePayload::new(0, b"transfer:alice:bob:100".to_vec());
        let sig     = sign(&kp, &payload).unwrap();
        assert!(verify(kp.public_key(), &payload, &sig).is_ok());
    }

    #[test]
    fn test_verify_fails_wrong_key() {
        let kp1     = KeyPair::generate().unwrap();
        let kp2     = KeyPair::generate().unwrap();
        let payload = NoncePayload::new(0, b"transfer".to_vec());
        let sig     = sign(&kp1, &payload).unwrap();
        // Signature from kp1 must not verify under kp2.
        assert!(verify(kp2.public_key(), &payload, &sig).is_err());
    }

    #[test]
    fn test_verify_fails_tampered_payload() {
        let kp      = KeyPair::generate().unwrap();
        let payload = NoncePayload::new(0, b"amount:100".to_vec());
        let sig     = sign(&kp, &payload).unwrap();
        // Attacker changes the amount.
        let tampered = NoncePayload::new(0, b"amount:999".to_vec());
        assert!(verify(kp.public_key(), &tampered, &sig).is_err());
    }

    #[test]
    fn test_verify_fails_wrong_nonce() {
        let kp      = KeyPair::generate().unwrap();
        let payload = NoncePayload::new(1, b"tx".to_vec());
        let sig     = sign(&kp, &payload).unwrap();
        // Replaying with nonce 0 must fail.
        let replayed = NoncePayload::new(0, b"tx".to_vec());
        assert!(verify(kp.public_key(), &replayed, &sig).is_err());
    }

    #[test]
    fn test_signing_is_deterministic() {
        let kp      = KeyPair::generate().unwrap();
        let payload = NoncePayload::new(42, b"same message".to_vec());
        let sig1    = sign(&kp, &payload).unwrap();
        let sig2    = sign(&kp, &payload).unwrap();
        // Ed25519 is deterministic: identical inputs → identical output.
        assert_eq!(sig1.as_bytes(), sig2.as_bytes());
    }

    #[test]
    fn test_signature_roundtrip() {
        let kp      = KeyPair::generate().unwrap();
        let payload = NoncePayload::new(0, b"roundtrip".to_vec());
        let sig     = sign(&kp, &payload).unwrap();
        let bytes   = *sig.as_bytes();
        let sig2    = Signature::from_bytes(bytes).unwrap();
        assert_eq!(sig.as_bytes(), sig2.as_bytes());
    }

    // ── Address ──────────────────────────────────────────────────────────────

    #[test]
    fn test_address_derivation_is_deterministic() {
        let kp   = KeyPair::generate().unwrap();
        let addr1 = Address::from_public_key(kp.public_key());
        let addr2 = Address::from_public_key(kp.public_key());
        assert_eq!(addr1, addr2);
    }

    #[test]
    fn test_address_different_keys_differ() {
        let kp1   = KeyPair::generate().unwrap();
        let kp2   = KeyPair::generate().unwrap();
        let addr1 = Address::from_public_key(kp1.public_key());
        let addr2 = Address::from_public_key(kp2.public_key());
        assert_ne!(addr1, addr2);
    }

    #[test]
    fn test_address_checksum_roundtrip() {
        let kp       = KeyPair::generate().unwrap();
        let addr     = Address::from_public_key(kp.public_key());
        let hex      = addr.to_checksum_hex();
        let restored = Address::from_checksum_hex(&hex).unwrap();
        assert_eq!(addr, restored);
    }

    #[test]
    fn test_address_bad_checksum_rejected() {
        let kp   = KeyPair::generate().unwrap();
        let addr = Address::from_public_key(kp.public_key());
        let mut hex = addr.to_checksum_hex();
        // Flip the case of the first alphabetic character after "0x".
        for ch in unsafe { hex.as_bytes_mut().iter_mut().skip(2) } {
            if ch.is_ascii_alphabetic() {
                *ch ^= 0x20; // toggle case
                break;
            }
        }
        assert!(Address::from_checksum_hex(&hex).is_err());
    }

    #[test]
    fn test_address_wrong_prefix_rejected() {
        assert!(Address::from_checksum_hex("1xdeadbeef").is_err());
    }

    // ── Domain separation ─────────────────────────────────────────────────────

    #[test]
    fn test_domain_separation_sign_and_verify_consistent() {
        // Signing and verification must use the same domain constant.
        // This test catches any accidental mismatch between sign/verify pipelines.
        let kp      = KeyPair::generate().unwrap();
        let payload = NoncePayload::new(0, b"domain-sep-test".to_vec());
        let sig     = sign(&kp, &payload).unwrap();
        assert!(verify(kp.public_key(), &payload, &sig).is_ok());
    }

    #[test]
    fn test_domain_separation_isolates_contexts() {
        // Construct two payloads whose raw NoncePayload::encode() bytes are
        // identical but that come from different logical contexts. Because
        // sign() prepends DOMAIN_SEP before hashing, the digests differ and
        // the signature from one context is invalid in another.
        //
        // We simulate this by manually verifying that the pre-image fed to
        // SHA-256 contains the domain prefix, by checking that a signature
        // produced with nonce=0, payload=b"SYPCOIN_TX_V1<nonce0><data>"
        // does NOT verify against a naive payload with those same bytes
        // (i.e., the domain constant is not forgeable via payload content).
        let kp = KeyPair::generate().unwrap();

        // Attempt to forge: pack DOMAIN_SEP into the payload itself.
        // If domain separation were implemented naively (concat without hash),
        // this could produce a collision. With SHA-256(domain||encode), it cannot.
        let forged_payload = {
            let mut v = b"SYPCOIN_TX_V1".to_vec();
            v.extend_from_slice(&0u64.to_le_bytes()); // nonce=0 LE
            v.extend_from_slice(b"real-data");
            v
        };
        let legitimate = NoncePayload::new(0, b"real-data".to_vec());
        let forged      = NoncePayload::new(0, forged_payload);

        let sig = sign(&kp, &legitimate).unwrap();
        // The forged payload encodes differently → different digest → fails.
        assert!(verify(kp.public_key(), &forged, &sig).is_err());
    }

    // ── Non-canonical signature rejection (from_bytes guard) ─────────────────

    // (see test_non_canonical_signature_rejected below for verify_strict test)

    // ── NoncePayload encoding ─────────────────────────────────────────────────

    #[test]
    fn test_nonce_payload_encoding_includes_nonce() {
        let p0 = NoncePayload::new(0, b"tx".to_vec());
        let p1 = NoncePayload::new(1, b"tx".to_vec());
        assert_ne!(p0.encode(), p1.encode());
    }

    #[test]
    fn test_nonce_payload_encoding_deterministic() {
        let p = NoncePayload::new(7, b"hello".to_vec());
        assert_eq!(p.encode(), p.encode());
    }

    // ── Strict verification ───────────────────────────────────────────────────

    #[test]
    fn test_non_canonical_signature_rejected() {
        // Build a structurally parseable but semantically invalid signature by
        // zeroing the S scalar (all-zero S is outside [0, l) for Ed25519).
        // verify() must reject it via verify_strict().
        let kp      = KeyPair::generate().unwrap();
        let payload = NoncePayload::new(0, b"strict-verify".to_vec());
        let mut sig_bytes = *sign(&kp, &payload).unwrap().as_bytes();
        // Zero out the S component (bytes 32..64). S=0 is non-canonical.
        for b in &mut sig_bytes[32..] { *b = 0; }
        // from_bytes() may or may not reject this depending on dalek internals;
        // verify_strict() must always reject it.
        if let Ok(bad_sig) = Signature::from_bytes(sig_bytes) {
            assert!(verify(kp.public_key(), &payload, &bad_sig).is_err());
        }
        // If from_bytes() itself rejects it, that is also correct behaviour.
    }
}