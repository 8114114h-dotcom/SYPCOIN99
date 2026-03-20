// cache.rs — LRU cache for recently-accessed blocks and headers.
//
// Reduces disk I/O for the most recently accessed blocks (tip, parent, etc.)
// Same LRU implementation pattern as state/cache.rs.

use std::collections::VecDeque;

use block::{Block, BlockHeader};
use crypto::HashDigest;

pub struct StorageCache {
    capacity:       usize,
    blocks:         VecDeque<(String, Block)>,
    headers:        VecDeque<(String, BlockHeader)>,
}

impl StorageCache {
    pub fn new(capacity: usize) -> Self {
        StorageCache {
            capacity,
            blocks:  VecDeque::with_capacity(capacity),
            headers: VecDeque::with_capacity(capacity),
        }
    }

    // ── Block cache ───────────────────────────────────────────────────────────

    pub fn get_block(&mut self, hash: &HashDigest) -> Option<Block> {
        let key = hex::encode(hash.as_bytes());
        if let Some(pos) = self.blocks.iter().position(|(k, _)| k == &key) {
            let entry = self.blocks.remove(pos).unwrap();
            self.blocks.push_front(entry.clone());
            return Some(entry.1);
        }
        None
    }

    pub fn insert_block(&mut self, block: Block) {
        let key = hex::encode(block.hash().as_bytes());
        self.blocks.retain(|(k, _)| k != &key);
        if self.blocks.len() >= self.capacity {
            self.blocks.pop_back();
        }
        self.blocks.push_front((key, block));
    }

    pub fn invalidate_block(&mut self, hash: &HashDigest) {
        let key = hex::encode(hash.as_bytes());
        self.blocks.retain(|(k, _)| k != &key);
        self.headers.retain(|(k, _)| k != &key);
    }

    // ── Header cache ──────────────────────────────────────────────────────────

    pub fn get_header(&mut self, hash: &HashDigest) -> Option<BlockHeader> {
        let key = hex::encode(hash.as_bytes());
        if let Some(pos) = self.headers.iter().position(|(k, _)| k == &key) {
            let entry = self.headers.remove(pos).unwrap();
            self.headers.push_front(entry.clone());
            return Some(entry.1);
        }
        None
    }

    pub fn insert_header(&mut self, hash: HashDigest, header: BlockHeader) {
        let key = hex::encode(hash.as_bytes());
        self.headers.retain(|(k, _)| k != &key);
        if self.headers.len() >= self.capacity {
            self.headers.pop_back();
        }
        self.headers.push_front((key, header));
    }

    pub fn clear(&mut self) {
        self.blocks.clear();
        self.headers.clear();
    }
}
