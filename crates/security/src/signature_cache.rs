// signature_cache.rs — LRU cache for verified signatures.
//
// Ed25519 verification is relatively expensive (~50µs per signature).
// For high-throughput nodes, caching recently-verified signatures
// avoids re-verifying the same transaction multiple times as it
// propagates through the mempool and block validation pipeline.
//
// Cache key: (pubkey_bytes[0..8], tx_id[0..32])
//   Using only 8 bytes of the public key as part of the key reduces
//   memory overhead while keeping collision probability negligible
//   (2^64 collision space for the public key prefix).
//
// Security:
//   • Cache only stores `true` (valid) results from verify().
//     We never cache `false` — an invalid signature might later be
//     replaced by a valid one with the same tx_id (unlikely but safe).
//   • Cache is invalidated on any reorg deeper than 1 block.

use std::collections::VecDeque;

/// Cache key: pubkey prefix (8 bytes) + full tx_id (32 bytes).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct CacheKey {
    pub pubkey_prefix: [u8; 8],
    pub tx_id:         [u8; 32],
}

impl CacheKey {
    pub fn new(pubkey: &[u8; 32], tx_id: &[u8; 32]) -> Self {
        let mut prefix = [0u8; 8];
        prefix.copy_from_slice(&pubkey[..8]);
        CacheKey { pubkey_prefix: prefix, tx_id: *tx_id }
    }
}

/// LRU signature verification cache.
pub struct SignatureCache {
    entries:  VecDeque<(CacheKey, bool)>,
    capacity: usize,
    hits:     u64,
    misses:   u64,
}

impl SignatureCache {
    pub fn new(capacity: usize) -> Self {
        SignatureCache {
            entries:  VecDeque::with_capacity(capacity),
            capacity,
            hits:     0,
            misses:   0,
        }
    }

    /// Look up a cached verification result.
    ///
    /// Moves the entry to the front (MRU) on hit.
    pub fn get(&mut self, key: &CacheKey) -> Option<bool> {
        if let Some(pos) = self.entries.iter().position(|(k, _)| k == key) {
            let entry = self.entries.remove(pos).unwrap();
            self.entries.push_front(entry.clone());
            self.hits += 1;
            return Some(entry.1);
        }
        self.misses += 1;
        None
    }

    /// Store a verification result.
    ///
    /// Only `true` results are cached — we never cache invalid signatures.
    pub fn insert(&mut self, key: CacheKey, valid: bool) {
        if !valid {
            return; // never cache failures
        }
        // Remove existing entry for this key.
        self.entries.retain(|(k, _)| k != &key);
        // Evict LRU if at capacity.
        if self.entries.len() >= self.capacity {
            self.entries.pop_back();
        }
        self.entries.push_front((key, valid));
    }

    /// Cache hit rate as a fraction [0.0, 1.0].
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { self.hits as f64 / total as f64 }
    }

    pub fn hits(&self)   -> u64 { self.hits }
    pub fn misses(&self) -> u64 { self.misses }
    pub fn len(&self)    -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }

    /// Invalidate all cached entries (call after deep reorg).
    pub fn invalidate_all(&mut self) {
        self.entries.clear();
    }
}
