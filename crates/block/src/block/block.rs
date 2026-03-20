// block/block.rs — The Block type: header + body.

use serde::{Deserialize, Serialize};

use crypto::{Address, HashDigest};
use primitives::{BlockHeight, Timestamp};
use transaction::Transaction;

use crate::body::body::BlockBody;
use crate::header::header::BlockHeader;

/// A complete block: header + body.
///
/// # Invariants (enforced by BlockBuilder)
/// - `header.merkle_root == body.merkle_root()`
/// - `header.tx_count == body.tx_count()`
/// - `header.height == parent.height + 1` (except genesis)
/// - PoW: `header.hash() < target(header.difficulty)`
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub(crate) header: BlockHeader,
    pub(crate) body:   BlockBody,
}

impl Block {
    // ── Accessors ─────────────────────────────────────────────────────────────

    pub fn header(&self)       -> &BlockHeader   { &self.header }
    pub fn body(&self)         -> &BlockBody     { &self.body }
    pub fn height(&self)       -> BlockHeight    { self.header.height() }
    pub fn parent_hash(&self)  -> &HashDigest    { self.header.parent_hash() }
    pub fn merkle_root(&self)  -> &HashDigest    { self.header.merkle_root() }
    pub fn state_root(&self)   -> &HashDigest    { self.header.state_root() }
    pub fn timestamp(&self)    -> Timestamp      { self.header.timestamp() }
    pub fn difficulty(&self)   -> u64            { self.header.difficulty() }
    pub fn nonce(&self)        -> u64            { self.header.nonce() }
    pub fn miner(&self)        -> &Address       { self.header.miner() }
    pub fn transactions(&self) -> &[Transaction] { self.body.transactions() }
    pub fn tx_count(&self)     -> u32            { self.body.tx_count() }
    pub fn is_genesis(&self)   -> bool           { self.header.is_genesis() }

    /// Hash of this block (= hash of its header).
    pub fn hash(&self) -> HashDigest {
        self.header.hash()
    }

    /// Total serialized size in bytes (header + body).
    pub fn size_bytes(&self) -> usize {
        self.header.to_bytes().len() + self.body.size_bytes()
    }

    /// Look up a transaction by ID.
    pub fn get_transaction(&self, tx_id: &HashDigest) -> Option<&Transaction> {
        self.body.get(tx_id)
    }

    /// Allow the miner (consensus crate) to update the nonce.
    #[allow(dead_code)]
    pub fn set_nonce(&mut self, nonce: u64) {
        self.header.set_nonce(nonce);
    }
}

impl std::fmt::Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Block[{}] height={} txs={} diff={} miner={}",
            hex::encode(&self.hash().as_bytes()[..8]),
            self.height(),
            self.tx_count(),
            self.difficulty(),
            self.miner().to_checksum_hex(),
        )
    }
}
