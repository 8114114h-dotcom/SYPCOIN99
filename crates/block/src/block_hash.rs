// block_hash.rs — Block hash computation.
//
// Security decisions:
//
//   DOUBLE SHA-256
//   • block_hash = SHA-256(SHA-256(header_bytes))
//     Bitcoin uses double-SHA-256 to protect against length-extension attacks
//     on the Merkle tree and to provide additional security margin.
//     We follow the same convention for the block hash.
//
//   DOMAIN SEPARATION
//   • A domain prefix "SYPCOIN_BLOCK_V1" is prepended to the header bytes
//     before hashing. This ensures a block hash can never collide with a
//     transaction hash, address, or state root, even if the raw bytes match.
//
//   HEADER-ONLY HASH
//   • Only the header is hashed — NOT the body (transactions).
//     The transactions are committed via the merkle_root field in the header.
//     This allows efficient header-chain verification without downloading
//     all transaction data.

use crypto::{HashDigest, sha256};
use crate::header::header::BlockHeader;

const BLOCK_HASH_DOMAIN: &[u8] = b"SYPCOIN_BLOCK_V1";

/// Compute the canonical hash of a block header.
///
/// `hash = SHA-256(SHA-256(DOMAIN || header_bytes))`
pub fn compute_block_hash(header: &BlockHeader) -> HashDigest {
    let header_bytes = header.to_bytes();
    let mut pre = Vec::with_capacity(BLOCK_HASH_DOMAIN.len() + header_bytes.len());
    pre.extend_from_slice(BLOCK_HASH_DOMAIN);
    pre.extend_from_slice(&header_bytes);

    // Double SHA-256.
    let first  = sha256(&pre);
    sha256(first.as_bytes())
}

/// Convert a difficulty value to a 32-byte PoW target.
///
/// target = MAX_HASH / difficulty
///
/// A valid PoW requires: block_hash (as u256) < target
///
/// difficulty=1    → target = 0x00FFFFFF...  (very easy)
/// difficulty=1000 → target = 0x000FFFFF...  (harder)
///
/// We represent the target as a [u8; 32] big-endian integer.
pub fn difficulty_to_target(difficulty: u64) -> [u8; 32] {
    if difficulty == 0 {
        return [0xFF; 32]; // difficulty=0 means accept any hash
    }

    // We compute target = 2^256 / difficulty using u128 arithmetic
    // on the most significant bytes for a practical approximation.
    // This is sufficient for a PoW chain; a full 256-bit division
    // would require a bignum library.
    //
    // We set the target by computing how many leading zero bits are needed.
    // leading_zeros = log2(difficulty) (approximate).
    let leading_zeros = (difficulty as f64).log2() as usize;
    let leading_zero_bytes = leading_zeros / 8;
    let remaining_bits     = leading_zeros % 8;

    let mut target = [0xFFu8; 32];

    // Zero out the leading bytes.
    for i in 0..leading_zero_bytes.min(32) {
        target[i] = 0x00;
    }

    // Partially zero the next byte.
    if leading_zero_bytes < 32 {
        target[leading_zero_bytes] = 0xFF >> remaining_bits;
    }

    target
}

/// Returns `true` if `hash` meets the required difficulty target.
///
/// The hash (interpreted as a big-endian 256-bit integer) must be
/// strictly less than the target.
pub fn meets_target(hash: &HashDigest, target: &[u8; 32]) -> bool {
    // Lexicographic comparison of big-endian byte arrays is equivalent
    // to numeric comparison of the 256-bit integers they represent.
    hash.as_bytes() < target
}
