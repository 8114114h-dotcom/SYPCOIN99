// gossip/broadcast.rs — Transaction and block broadcasting.
//
// Gossip protocol: when a node receives a new tx or block, it broadcasts
// it to all connected peers (except the one it came from).
//
// Anti-flood: we track which tx_ids and block hashes have been seen
// recently to avoid re-broadcasting the same item multiple times.

use std::collections::HashSet;

use crypto::HashDigest;

/// Tracks recently-seen tx and block hashes to avoid redundant broadcasts.
pub struct BroadcastTracker {
    seen_txs:    HashSet<String>,
    seen_blocks: HashSet<String>,
    max_entries: usize,
}

impl BroadcastTracker {
    pub fn new(max_entries: usize) -> Self {
        BroadcastTracker {
            seen_txs:    HashSet::new(),
            seen_blocks: HashSet::new(),
            max_entries,
        }
    }

    /// Returns `true` if this tx has NOT been seen before (and records it).
    pub fn is_new_tx(&mut self, tx_id: &HashDigest) -> bool {
        let key = hex::encode(tx_id.as_bytes());
        if self.seen_txs.contains(&key) {
            return false;
        }
        if self.seen_txs.len() >= self.max_entries {
            // Evict oldest — for simplicity, clear half when full.
            let keep: HashSet<_> = self.seen_txs.iter()
                .skip(self.seen_txs.len() / 2)
                .cloned()
                .collect();
            self.seen_txs = keep;
        }
        self.seen_txs.insert(key);
        true
    }

    /// Returns `true` if this block has NOT been seen before (and records it).
    pub fn is_new_block(&mut self, hash: &HashDigest) -> bool {
        let key = hex::encode(hash.as_bytes());
        if self.seen_blocks.contains(&key) {
            return false;
        }
        if self.seen_blocks.len() >= self.max_entries {
            let keep: HashSet<_> = self.seen_blocks.iter()
                .skip(self.seen_blocks.len() / 2)
                .cloned()
                .collect();
            self.seen_blocks = keep;
        }
        self.seen_blocks.insert(key);
        true
    }

    pub fn clear(&mut self) {
        self.seen_txs.clear();
        self.seen_blocks.clear();
    }
}
