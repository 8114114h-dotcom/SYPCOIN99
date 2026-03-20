// address/address.rs — Address derivation and checksum encoding.
//
// Security decisions:
//
//   ADDRESS DERIVATION
//   • address = SHA-256(b"SYPCOIN_ADDR_V1" || pubkey_bytes)[0..20]
//     A domain prefix is prepended so that the address hash is in a distinct
//     domain from transaction signing. This prevents any hypothetical
//     cross-context pre-image attack where a crafted transaction digest
//     could match an address pre-image.
//   • Truncating to 20 bytes yields 160 bits of pre-image resistance —
//     sufficient against brute-force reversal given SHA-256's collision
//     resistance of 2^128.
//
//   CHECKSUM ALGORITHM (EIP-55 variant, SHA-256)
//   • 1. Lowercase-hex-encode the 20 address bytes → 40 ASCII chars.
//   • 2. SHA-256(those 40 bytes) → 32-byte checksum digest.
//   • 3. For hex char at position i:
//          byte = digest[i/2]
//          bit  = MSB of byte (i even) or bit-3 of byte (i odd)
//          if bit set AND char is alpha → uppercase, else → lowercase
//   • A single-character typo changes ≥1 checksum bit with probability
//     ≥ 50%, catching >99.9% of one-character address errors.
//
//   CONSTANT-TIME CHECKSUM COMPARISON
//   • from_checksum_hex() uses a byte-by-byte XOR accumulator to compare
//     the supplied string against the expected checksum hex. This runs in
//     time proportional to the string length regardless of where a mismatch
//     occurs, preventing a timing side-channel that could leak information
//     about partial checksum matches.

use serde::{Deserialize, Serialize};

use crate::error::CryptoError;
use crate::hashing::hash::sha256;
use crate::keys::keypair::PublicKey;

/// Domain separator for address derivation.
/// Distinct from DOMAIN_SEP in signer.rs — these are different hash contexts.
const ADDR_DOMAIN: &[u8] = b"SYPCOIN_ADDR_V1";

/// A 20-byte account address derived from a public key.
///
/// Always displayed and parsed in checksum-hex form (`0x` prefix, 42 chars total).
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct Address(pub(crate) [u8; 20]);

impl Address {
    /// Derive an address from a public key.
    ///
    /// `address = SHA-256(b"SYPCOIN_ADDR_V1" || pubkey_bytes)[0..20]`
    ///
    /// The domain prefix separates address hashing from all other hash contexts
    /// in the protocol.
    pub fn from_public_key(pk: &PublicKey) -> Self {
        // Pre-image: ADDR_DOMAIN || pubkey_bytes
        let mut pre = Vec::with_capacity(ADDR_DOMAIN.len() + 32);
        pre.extend_from_slice(ADDR_DOMAIN);
        pre.extend_from_slice(pk.as_bytes());

        let digest = sha256(&pre);
        let mut bytes = [0u8; 20];
        bytes.copy_from_slice(&digest.as_bytes()[..20]);
        Address(bytes)
    }

    /// Encode this address as a checksummed hex string.
    ///
    /// Format: `"0x"` followed by 40 hex characters with EIP-55-style
    /// mixed-case checksum (SHA-256 variant).
    pub fn to_checksum_hex(&self) -> String {
        let lower_hex = hex::encode(self.0); // 40 lowercase hex chars
        let checksum  = sha256(lower_hex.as_bytes());
        let cs_bytes  = checksum.as_bytes();

        let mut out = String::with_capacity(42);
        out.push_str("0x");

        for (i, ch) in lower_hex.chars().enumerate() {
            // Each byte of the checksum digest covers two hex characters.
            // Even positions use the MSB; odd positions use bit 3.
            let byte    = cs_bytes[i / 2];
            let bit_set = if i % 2 == 0 {
                byte & 0x80 != 0   // MSB of byte → controls char at even index
            } else {
                byte & 0x08 != 0   // bit 3 of byte → controls char at odd index
            };

            if bit_set && ch.is_ascii_alphabetic() {
                out.push(ch.to_ascii_uppercase());
            } else {
                out.push(ch);
            }
        }

        out
    }

    /// Parse and validate a checksum hex address string.
    ///
    /// Accepts `"0x"` prefix (required). Rejects incorrect checksum casing,
    /// wrong length, or non-hex characters.
    ///
    /// The comparison against the expected checksum is performed in constant
    /// time (XOR accumulator) to prevent timing side-channels.
    pub fn from_checksum_hex(s: &str) -> Result<Self, CryptoError> {
        // Must start with "0x" and be exactly 42 characters.
        let hex_part = s.strip_prefix("0x").ok_or(CryptoError::InvalidAddress)?;
        if hex_part.len() != 40 {
            return Err(CryptoError::InvalidAddress);
        }

        // Validate hex characters and decode raw bytes.
        let raw = hex::decode(hex_part).map_err(|_| CryptoError::InvalidAddress)?;
        let mut bytes = [0u8; 20];
        bytes.copy_from_slice(&raw);

        // Reconstruct the expected checksummed hex string.
        let addr     = Address(bytes);
        let expected = addr.to_checksum_hex(); // always "0x" + 40 chars

        // Constant-time comparison: XOR every byte of the supplied string
        // against the expected string, accumulate differences into `diff`.
        // `diff` is zero iff the strings are identical.
        // We compare the full 42-character strings (including "0x" prefix).
        let supplied  = s.as_bytes();
        let reference = expected.as_bytes();

        // Both must be the same length at this point (42 bytes); we already
        // checked hex_part.len() == 40 so s.len() == 42.
        debug_assert_eq!(supplied.len(), reference.len());

        let diff = supplied
            .iter()
            .zip(reference.iter())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b));

        if diff != 0 {
            return Err(CryptoError::InvalidChecksum);
        }

        Ok(addr)
    }

    /// Raw address bytes. Available for hashing in block headers, etc.
    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }
}

impl Address {
    pub fn from_raw_bytes(bytes: [u8; 20]) -> Self {
        Address(bytes)
    }
}
