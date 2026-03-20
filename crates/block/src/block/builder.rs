// block/builder.rs — BlockBuilder.
//
// The only way to construct a Block. Enforces:
//   1. All required fields are present.
//   2. merkle_root is computed from transactions — never supplied externally.
//   3. tx_count matches the transaction list.
//   4. Block size is within limits.
//
// The PoW nonce starts at 0. The consensus/miner crate increments it
// via Block::set_nonce() during the mining loop.

use crypto::{Address, HashDigest};
use primitives::{BlockHeight, Timestamp};
use transaction::Transaction;

use crate::block::block::Block;
use crate::body::body::BlockBody;
use crate::error::BlockError;
use crate::header::header::{BlockHeader, BLOCK_VERSION};
use crate::size_limit::check_block_size;

/// Builder for constructing a [`Block`].
#[derive(Default)]
pub struct BlockBuilder {
    height:       Option<BlockHeight>,
    parent_hash:  Option<HashDigest>,
    state_root:   Option<HashDigest>,
    miner:        Option<Address>,
    difficulty:   Option<u64>,
    timestamp:    Option<Timestamp>,
    transactions: Option<Vec<Transaction>>,
    nonce:        u64,  // starts at 0
}

impl BlockBuilder {
    pub fn new() -> Self {
        BlockBuilder::default()
    }

    pub fn height(mut self, h: BlockHeight) -> Self {
        self.height = Some(h);
        self
    }

    pub fn parent_hash(mut self, hash: HashDigest) -> Self {
        self.parent_hash = Some(hash);
        self
    }

    pub fn state_root(mut self, root: HashDigest) -> Self {
        self.state_root = Some(root);
        self
    }

    pub fn miner(mut self, addr: Address) -> Self {
        self.miner = Some(addr);
        self
    }

    pub fn difficulty(mut self, d: u64) -> Self {
        self.difficulty = Some(d);
        self
    }

    pub fn timestamp(mut self, ts: Timestamp) -> Self {
        self.timestamp = Some(ts);
        self
    }

    pub fn transactions(mut self, txs: Vec<Transaction>) -> Self {
        self.transactions = Some(txs);
        self
    }

    /// Set initial nonce (default 0). The miner will iterate from here.
    pub fn nonce(mut self, n: u64) -> Self {
        self.nonce = n;
        self
    }

    /// Build the block.
    ///
    /// Computes merkle_root automatically from the transaction list.
    /// The block is returned with nonce=0 (or the set nonce).
    /// The consensus layer increments the nonce during mining.
    pub fn build(self) -> Result<Block, BlockError> {
        // ── Unwrap required fields ─────────────────────────────────────────────
        let height = self.height
            .ok_or_else(|| BlockError::MissingField("height".into()))?;
        let parent_hash = self.parent_hash
            .ok_or_else(|| BlockError::MissingField("parent_hash".into()))?;
        let state_root = self.state_root
            .ok_or_else(|| BlockError::MissingField("state_root".into()))?;
        let miner = self.miner
            .ok_or_else(|| BlockError::MissingField("miner".into()))?;
        let difficulty = self.difficulty
            .ok_or_else(|| BlockError::MissingField("difficulty".into()))?;
        let timestamp = self.timestamp.unwrap_or_else(Timestamp::now);
        let transactions = self.transactions.unwrap_or_default();

        // ── Validate ──────────────────────────────────────────────────────────
        if difficulty == 0 {
            return Err(BlockError::InvalidDifficulty);
        }

        // ── Build body ────────────────────────────────────────────────────────
        let body = BlockBody::new(transactions)?;

        // ── Compute merkle root from body ─────────────────────────────────────
        let merkle_root = body.merkle_root();
        let tx_count    = body.tx_count();

        // ── Build header ──────────────────────────────────────────────────────
        let header = BlockHeader {
            version: BLOCK_VERSION,
            height,
            parent_hash,
            merkle_root,
            state_root,
            timestamp,
            difficulty,
            nonce: self.nonce,
            miner,
            tx_count,
        };

        let block = Block { header, body };

        // ── Final size check ──────────────────────────────────────────────────
        check_block_size(block.size_bytes())?;

        Ok(block)
    }
}
