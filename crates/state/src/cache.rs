// cache.rs — Simple LRU-style account cache.
//
// Purpose: avoid repeated lookups from the AccountStore (and later from disk)
// for hot accounts (e.g. the miner address, high-traffic accounts).
//
// Design:
//   • Capacity-bounded: when full, evicts the least-recently-used entry.
//   • Cache is invalidated on every write to that account.
//   • The cache is a read-through layer — misses fall back to AccountStore.
//   • NOT serialized — rebuilt from AccountStore on node restart.
//
// We implement a simple LRU using a VecDeque of (key, value) pairs.
// For production, swap this with the `lru` crate.

use std::collections::VecDeque;

use crypto::Address;
use crate::account::account::Account;

/// A capacity-bounded LRU cache for Account objects.
#[allow(dead_code)]
pub struct StateCache {
    capacity: usize,
    entries:  VecDeque<(String, Account)>, // (checksum_hex, account)
}

impl StateCache {
    /// Create a new cache with the given capacity.
    pub fn new(capacity: usize) -> Self {
        StateCache {
            capacity,
            entries: VecDeque::with_capacity(capacity),
        }
    }

    /// Look up an account by address.
    ///
    /// Moves the entry to the front (most-recently-used) on hit.
    pub fn get(&mut self, addr: &Address) -> Option<Account> {
        let key = addr.to_checksum_hex();
        if let Some(pos) = self.entries.iter().position(|(k, _)| k == &key) {
            let entry = self.entries.remove(pos).unwrap();
            self.entries.push_front(entry.clone());
            Some(entry.1)
        } else {
            None
        }
    }

    /// Insert or update an account in the cache.
    ///
    /// If the cache is full, evicts the least-recently-used entry (back).
    pub fn insert(&mut self, account: Account) {
        let key = account.address().to_checksum_hex();

        // Remove existing entry for this key if present.
        if let Some(pos) = self.entries.iter().position(|(k, _)| k == &key) {
            self.entries.remove(pos);
        }

        // Evict LRU if at capacity.
        if self.entries.len() >= self.capacity {
            self.entries.pop_back();
        }

        // Insert at front (most-recently-used).
        self.entries.push_front((key, account));
    }

    /// Invalidate (remove) a cached account.
    ///
    /// Must be called whenever an account is mutated in AccountStore.
    pub fn invalidate(&mut self, addr: &Address) {
        let key = addr.to_checksum_hex();
        self.entries.retain(|(k, _)| k != &key);
    }

    /// Clear all cached entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
