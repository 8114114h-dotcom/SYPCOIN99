// body/body.rs — Block body (transaction list).
//
// The body holds all transactions included in the block.
// It is separate from the header so that:
//   1. Header-chain verification works without downloading transactions.
//   2. The body can be transmitted and stored independently.

use std::collections::HashSet;
use serde::{Deserialize, Serialize};

use crypto::HashDigest;
use transaction::Transaction;

use crate::error::BlockError;
use crate::merkle::tree::compute_merkle_root;
use crate::size_limit::check_tx_count;

/// The body of a block — an ordered list of validated transactions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockBody {
    transactions: Vec<Transaction>,
}

impl BlockBody {
    /// Construct a block body from a list of transactions.
    ///
    /// Validates:
    /// - tx_count ≤ MAX_TX_PER_BLOCK
    /// - No duplicate tx_ids
    pub fn new(transactions: Vec<Transaction>) -> Result<Self, BlockError> {
        check_tx_count(transactions.len() as u32)?;

        // Detect duplicates.
        let mut seen = HashSet::new();
        for tx in &transactions {
            if !seen.insert(tx.tx_id().as_bytes()) {
                return Err(BlockError::DuplicateTransaction);
            }
        }

        Ok(BlockBody { transactions })
    }

    /// Construct an empty body (for genesis block).
    pub fn empty() -> Self {
        BlockBody { transactions: vec![] }
    }

    // ── Accessors ─────────────────────────────────────────────────────────────

    /// All transactions in this block body.
    pub fn transactions(&self) -> &[Transaction] {
        &self.transactions
    }

    /// Number of transactions.
    pub fn tx_count(&self) -> u32 {
        self.transactions.len() as u32
    }

    /// Returns `true` if the body contains no transactions.
    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    /// Compute the Merkle root of the transactions.
    pub fn merkle_root(&self) -> HashDigest {
        compute_merkle_root(&self.transactions)
    }

    /// Approximate serialized size of the body in bytes.
    pub fn size_bytes(&self) -> usize {
        // Each transaction is at most MAX_TX_SIZE_BYTES.
        // We use the actual to_bytes() size of each tx.
        self.transactions.iter().map(|tx| tx.size_bytes()).sum()
    }

    /// Look up a transaction by its ID.
    pub fn get(&self, tx_id: &HashDigest) -> Option<&Transaction> {
        self.transactions
            .iter()
            .find(|tx| tx.tx_id().as_bytes() == tx_id.as_bytes())
    }

    /// Returns `true` if the body contains a transaction with this ID.
    pub fn contains(&self, tx_id: &HashDigest) -> bool {
        self.get(tx_id).is_some()
    }
}
