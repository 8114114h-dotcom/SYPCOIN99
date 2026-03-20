// replay_protection.rs — Sliding-window replay attack prevention.
//
// Tracks the last `window_size` tx_ids seen by this node.
// A transaction whose tx_id is already in the window is rejected as a replay.
//
// Design:
//   • Uses a VecDeque as a circular buffer: oldest entry is popped when full.
//   • A HashSet provides O(1) membership checks.
//   • Window size default = 10,000 — covers ~2-3 blocks of transactions.
//
// Relationship with Nonce:
//   • The nonce in each transaction is the primary replay protection mechanism
//     (consensus layer enforces monotonic nonces per account).
//   • This window is a secondary layer: it catches replays within the current
//     mempool window even before nonce validation runs.

use std::collections::{HashSet, VecDeque};

use crypto::HashDigest;

use crate::error::SecurityError;

/// Default number of recent tx_ids to remember.
pub const DEFAULT_WINDOW_SIZE: usize = 10_000;

/// Sliding-window replay protection.
pub struct ReplayProtection {
    /// Ordered history of seen tx_ids (front = oldest).
    history:     VecDeque<HashDigest>,
    /// Fast membership check.
    seen_set:    HashSet<String>,
    window_size: usize,
}

impl ReplayProtection {
    pub fn new(window_size: usize) -> Self {
        ReplayProtection {
            history:     VecDeque::with_capacity(window_size),
            seen_set:    HashSet::with_capacity(window_size),
            window_size,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(DEFAULT_WINDOW_SIZE)
    }

    /// Check if a tx_id has been seen before. Does NOT record it.
    ///
    /// Returns `Err(ReplayDetected)` if the tx was seen in the window.
    pub fn check(&self, tx_id: &HashDigest) -> Result<(), SecurityError> {
        let key = hex::encode(tx_id.as_bytes());
        if self.seen_set.contains(&key) {
            return Err(SecurityError::ReplayDetected { tx_id: key });
        }
        Ok(())
    }

    /// Record a tx_id as seen.
    ///
    /// If the window is full, the oldest entry is evicted.
    pub fn mark_seen(&mut self, tx_id: HashDigest) {
        let key = hex::encode(tx_id.as_bytes());
        if self.seen_set.contains(&key) {
            return; // already tracked
        }
        // Evict oldest if at capacity.
        if self.history.len() >= self.window_size {
            if let Some(old) = self.history.pop_front() {
                self.seen_set.remove(&hex::encode(old.as_bytes()));
            }
        }
        self.history.push_back(tx_id);
        self.seen_set.insert(key);
    }

    /// Check AND record in one step.
    ///
    /// Returns `Err` if the tx was already seen. On success, records it.
    pub fn check_and_mark(&mut self, tx_id: HashDigest) -> Result<(), SecurityError> {
        self.check(&tx_id)?;
        self.mark_seen(tx_id);
        Ok(())
    }

    pub fn is_seen(&self, tx_id: &HashDigest) -> bool {
        self.seen_set.contains(&hex::encode(tx_id.as_bytes()))
    }

    pub fn window_size(&self) -> usize { self.window_size }
    pub fn seen_count(&self)  -> usize { self.history.len() }

    /// Clear all recorded tx_ids (e.g. after a deep reorg).
    pub fn clear(&mut self) {
        self.history.clear();
        self.seen_set.clear();
    }
}
