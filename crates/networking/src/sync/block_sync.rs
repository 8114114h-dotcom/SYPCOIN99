// sync/block_sync.rs — Full block synchronisation.
//
// Phase 2 of sync: download full blocks for headers we already verified.

use std::collections::VecDeque;

use block::{Block, BlockHeader};
use crypto::HashDigest;

use crate::protocol::messages::{NetworkMessage, MAX_BLOCKS_PER_MSG};

/// State for an in-progress block download session.
pub struct BlockSync {
    /// Hashes we still need to download.
    pending: VecDeque<HashDigest>,
    /// Blocks received and ready to apply.
    pub received: Vec<Block>,
}

impl BlockSync {
    /// Create from a list of headers to download.
    pub fn from_headers(headers: &[BlockHeader]) -> Self {
        let pending = headers.iter().map(|h| h.hash()).collect();
        BlockSync {
            pending,
            received: Vec::new(),
        }
    }

    /// Build a GetBlocks request for the next batch.
    pub fn next_request(&self) -> Option<NetworkMessage> {
        if self.pending.is_empty() {
            return None;
        }
        let hashes: Vec<_> = self.pending
            .iter()
            .take(MAX_BLOCKS_PER_MSG as usize)
            .cloned()
            .collect();
        Some(NetworkMessage::GetBlocks { hashes })
    }

    /// Process a received batch of blocks.
    pub fn on_blocks(&mut self, blocks: Vec<Block>) {
        for block in blocks {
            // Remove from pending.
            let hash = block.hash();
            self.pending.retain(|h| h.as_bytes() != hash.as_bytes());
            self.received.push(block);
        }
    }

    pub fn is_complete(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}
