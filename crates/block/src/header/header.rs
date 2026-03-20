// header/header.rs — Block header.
//
// The header is the minimal unit of chain verification.
// A node can validate the chain of headers without downloading transactions.
//
// Canonical serialization layout (all integers little-endian):
//   version(1) || height(8) || parent_hash(32) || merkle_root(32) ||
//   state_root(32) || timestamp(8) || difficulty(8) || nonce(8) ||
//   miner(20) || tx_count(4)
//
// Total: 1+8+32+32+32+8+8+8+20+4 = 153 bytes (fixed width, no length prefixes)
//
// CONSENSUS CRITICAL — this format must never change without a version bump.

use serde::{Deserialize, Serialize};

use crypto::{Address, HashDigest};
use primitives::{BlockHeight, Timestamp};

use crate::block_hash::compute_block_hash;

pub const BLOCK_VERSION: u8 = 1;

/// The block header — everything needed to verify the chain without transactions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Block format version.
    pub(crate) version:     u8,

    /// Height of this block in the chain (0 = genesis).
    pub(crate) height:      BlockHeight,

    /// Hash of the parent block header. All-zeros for genesis.
    pub(crate) parent_hash: HashDigest,

    /// Merkle root of all transactions in this block.
    pub(crate) merkle_root: HashDigest,

    /// Merkle root of the world state after applying this block.
    pub(crate) state_root:  HashDigest,

    /// Block creation timestamp (Unix milliseconds).
    pub(crate) timestamp:   Timestamp,

    /// Mining difficulty target.
    pub(crate) difficulty:  u64,

    /// Proof-of-Work nonce. Set by the miner.
    pub(crate) nonce:       u64,

    /// Address of the miner who produced this block.
    pub(crate) miner:       Address,

    /// Number of transactions in the block body.
    pub(crate) tx_count:    u32,
}

impl BlockHeader {
    // ── Accessors ─────────────────────────────────────────────────────────────

    pub fn version(&self)     -> u8            { self.version }
    pub fn height(&self)      -> BlockHeight   { self.height }
    pub fn parent_hash(&self) -> &HashDigest   { &self.parent_hash }
    pub fn merkle_root(&self) -> &HashDigest   { &self.merkle_root }
    pub fn state_root(&self)  -> &HashDigest   { &self.state_root }
    pub fn timestamp(&self)   -> Timestamp     { self.timestamp }
    pub fn difficulty(&self)  -> u64           { self.difficulty }
    pub fn nonce(&self)       -> u64           { self.nonce }
    pub fn miner(&self)       -> &Address      { &self.miner }
    pub fn tx_count(&self)    -> u32           { self.tx_count }

    /// Returns `true` if this is the genesis block (height == 0).
    pub fn is_genesis(&self) -> bool {
        self.height.is_genesis()
    }

    // ── Hash ──────────────────────────────────────────────────────────────────

    /// Compute the canonical hash of this header.
    ///
    /// `hash = SHA-256(SHA-256(DOMAIN || to_bytes()))`
    pub fn hash(&self) -> HashDigest {
        compute_block_hash(self)
    }

    // ── Serialization ─────────────────────────────────────────────────────────

    /// Canonical fixed-width byte representation.
    ///
    /// Used for hashing and PoW. CONSENSUS CRITICAL — must never change.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(153);
        buf.push(self.version);
        buf.extend_from_slice(&self.height.as_u64().to_le_bytes());
        buf.extend_from_slice(self.parent_hash.as_bytes());
        buf.extend_from_slice(self.merkle_root.as_bytes());
        buf.extend_from_slice(self.state_root.as_bytes());
        buf.extend_from_slice(&self.timestamp.as_millis().to_le_bytes());
        buf.extend_from_slice(&self.difficulty.to_le_bytes());
        buf.extend_from_slice(&self.nonce.to_le_bytes());
        buf.extend_from_slice(self.miner.as_bytes());
        buf.extend_from_slice(&self.tx_count.to_le_bytes());
        buf
    }

    /// Update the PoW nonce. Called by the miner during mining loop.
    ///
    /// `pub(crate)` — only the consensus crate increments the nonce.
    #[allow(dead_code)]
    pub fn set_nonce(&mut self, nonce: u64) {
        self.nonce = nonce;
    }
}

impl std::fmt::Display for BlockHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Header[{}] height={} txs={} diff={} nonce={}",
            hex::encode(&self.hash().as_bytes()[..8]),
            self.height,
            self.tx_count,
            self.difficulty,
            self.nonce,
        )
    }
}
