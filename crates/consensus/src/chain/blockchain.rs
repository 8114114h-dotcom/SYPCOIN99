// chain/blockchain.rs — The Blockchain: an ordered, indexed chain of blocks.
//
// Design:
//   • Blocks are stored in a Vec<Block> indexed by height.
//   • A HashMap<hash_hex, height> provides O(1) lookup by hash.
//   • The chain always starts with a genesis block.
//   • add_block() runs full consensus validation before appending.
//   • current_difficulty is tracked and updated at adjustment boundaries.

use std::collections::HashMap;
use block::{Block, BlockHeader};
use primitives::BlockHeight;

use crate::chain_rules::ChainRules;
use crate::error::ConsensusError;
use crate::pow::difficulty::should_adjust;
use primitives::constants::DIFFICULTY_ADJUSTMENT_INTERVAL;

/// The canonical blockchain: an ordered sequence of fully-validated blocks.
pub struct Blockchain {
    /// All blocks ordered by height (index == height).
    blocks:             Vec<Block>,
    /// O(1) lookup: block hash (hex) → height index.
    index:              HashMap<String, u64>,
    /// Current mining difficulty (for the NEXT block).
    current_difficulty: u64,
    /// Pending fork candidates exceeding MAX_REORG_DEPTH.
    /// Stored for operator review rather than silently discarded.
    /// Key: fork tip hash hex, Value: fork depth.
    deep_forks:         HashMap<String, u64>,
}

impl Blockchain {
    /// Create a new blockchain starting from a genesis block.
    pub fn new(genesis: Block, initial_difficulty: u64) -> Result<Self, ConsensusError> {
        if !genesis.is_genesis() {
            return Err(ConsensusError::InvalidGenesisBlock);
        }

        let hash_hex = hex::encode(genesis.hash().as_bytes());
        let mut index = HashMap::new();
        index.insert(hash_hex, 0u64);

        Ok(Blockchain {
            blocks: vec![genesis],
            index,
            current_difficulty: initial_difficulty,
            deep_forks:         HashMap::new(),
        })
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    /// The current chain tip (latest block).
    pub fn tip(&self) -> &Block {
        self.blocks.last().expect("chain always has at least genesis")
    }

    /// Current chain height.
    pub fn height(&self) -> BlockHeight {
        self.tip().height()
    }

    /// Get a block by its hash.
    pub fn get_block(&self, hash: &crypto::HashDigest) -> Option<&Block> {
        let hex = hex::encode(hash.as_bytes());
        self.index.get(&hex).and_then(|&h| self.blocks.get(h as usize))
    }

    /// Get a block by its height.
    pub fn get_block_at(&self, height: BlockHeight) -> Option<&Block> {
        self.blocks.get(height.as_u64() as usize)
    }

    /// Get the header at a specific height.
    pub fn get_header_at(&self, height: BlockHeight) -> Option<&BlockHeader> {
        self.get_block_at(height).map(|b| b.header())
    }

    /// Returns `true` if the chain contains a block with this hash.
    pub fn contains(&self, hash: &crypto::HashDigest) -> bool {
        self.index.contains_key(&hex::encode(hash.as_bytes()))
    }

    /// The difficulty that the NEXT block must use.
    pub fn current_difficulty(&self) -> u64 {
        self.current_difficulty
    }

    /// Number of blocks in the chain (including genesis).
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    // ── Mutation ──────────────────────────────────────────────────────────────

    /// Add a validated block to the chain.
    ///
    /// Runs full consensus validation before appending.
    /// Updates difficulty if this block triggers an adjustment.
    /// Add a block without validation — used only for replaying stored blocks.
    pub fn add_block_unchecked(&mut self, block: Block) {
        let height  = block.height().as_u64();
        let hash_hex = hex::encode(block.hash().as_bytes());
        self.blocks.push(block);
        self.index.insert(hash_hex, height);
        self.maybe_adjust_difficulty(height);
    }

    pub fn add_block(&mut self, block: Block) -> Result<(), ConsensusError> {
        // Reject duplicates.
        if self.contains(&block.hash()) {
            return Err(ConsensusError::BlockAlreadyKnown);
        }

        // ── Reorg trap prevention ─────────────────────────────────────────────
        // If this block does not extend our tip (parent hash mismatch),
        // it may be part of a deep fork. We record it but do NOT stall.
        // The node continues on the current chain while the operator
        // (or future sync logic) can inspect `deep_forks` and decide.
        let tip_hash = self.tip().hash();
        if block.parent_hash() != &tip_hash {
            // This block is on a different branch.
            let our_height   = self.height().as_u64();
            let block_height = block.height().as_u64();
            let depth        = our_height.saturating_sub(block_height);

            if depth > primitives::constants::MAX_REORG_DEPTH {
                let fork_id = hex::encode(block.hash().as_bytes());
                self.deep_forks.insert(fork_id, depth);
                eprintln!(
                    "[WARN] Deep fork detected: depth={} > MAX_REORG_DEPTH={} —                      recorded for review, NOT applying. Node continues on current chain.",
                    depth, primitives::constants::MAX_REORG_DEPTH
                );
                // Return specific error — NOT a fatal panic.
                return Err(ConsensusError::ReorgTooDeep {
                    depth,
                    max: primitives::constants::MAX_REORG_DEPTH,
                });
            }
        }

        let parent = self.tip().header().clone();

        // Full consensus validation.
        ChainRules::validate_block(&block, &parent, self.current_difficulty)?;

        // Append.
        let height  = block.height().as_u64();
        let hash_hex = hex::encode(block.hash().as_bytes());
        self.blocks.push(block);
        self.index.insert(hash_hex, height);

        // Adjust difficulty if needed.
        self.maybe_adjust_difficulty(height);

        Ok(())
    }

    /// Returns pending deep fork candidates (for operator inspection / sync).
    /// Key: fork tip hash hex, Value: fork depth in blocks.
    pub fn deep_forks(&self) -> &HashMap<String, u64> {
        &self.deep_forks
    }

    /// Clear the deep fork registry (call after operator review or re-sync).
    pub fn clear_deep_forks(&mut self) {
        self.deep_forks.clear();
    }

    // ── Internal ──────────────────────────────────────────────────────────────

    fn maybe_adjust_difficulty(&mut self, new_height: u64) {
        let tip_height = BlockHeight::new(new_height);
        if !should_adjust(&tip_height) {
            return;
        }

        // Find the block at the start of this adjustment interval.
        let interval_start_height = new_height
            .saturating_sub(DIFFICULTY_ADJUSTMENT_INTERVAL);

        let start_ms = self
            .get_block_at(BlockHeight::new(interval_start_height))
            .map(|b| b.timestamp().as_millis())
            .unwrap_or(0);

        let end_ms = self.tip().timestamp().as_millis();

        self.current_difficulty = ChainRules::next_difficulty(
            self.current_difficulty,
            &tip_height,
            start_ms,
            end_ms,
        );
    }
}
