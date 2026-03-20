use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use crypto::HashDigest;
use primitives::{Amount, Timestamp};

use crate::error::TransactionError;
use crate::mempool::eviction::EvictionPolicy;
use crate::mempool::ordering::FeeKey;
use crate::tx::transaction::Transaction;

#[derive(Clone, Debug)]
pub struct MempoolConfig {
    pub max_size:        usize,
    pub max_per_address: usize,
    pub min_fee:         Amount,
    pub tx_ttl_ms:       u64,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        use primitives::constants::{MAX_MEMPOOL_SIZE, MAX_MEMPOOL_TXS_PER_ADDRESS, MIN_TX_FEE_MICRO};
        use crate::tx::constants::TX_TTL_MS;
        MempoolConfig {
            max_size:        MAX_MEMPOOL_SIZE,
            max_per_address: MAX_MEMPOOL_TXS_PER_ADDRESS,
            min_fee:         Amount::from_micro(MIN_TX_FEE_MICRO).unwrap(),
            tx_ttl_ms:       TX_TTL_MS,
        }
    }
}

pub struct Mempool {
    config:     MempoolConfig,
    by_id:      HashMap<HashDigest, Arc<Transaction>>,
    by_fee:     BTreeMap<FeeKey, HashDigest>,
    by_address: HashMap<String, HashSet<HashDigest>>,
}

impl Mempool {
    pub fn new(config: MempoolConfig) -> Self {
        Mempool {
            config,
            by_id:      HashMap::new(),
            by_fee:     BTreeMap::new(),
            by_address: HashMap::new(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(MempoolConfig::default())
    }

    pub fn add(&mut self, tx: Transaction) -> Result<(), TransactionError> {
        // Capacity check with fee-based eviction
        if self.by_id.len() >= self.config.max_size {
            if let Some(min_fee_key) = self.by_fee.keys().last().cloned() {
                if let Some(min_id) = self.by_fee.get(&min_fee_key).cloned() {
                    if let Some(min_tx) = self.by_id.get(&min_id).cloned() {
                        match crate::mempool::eviction::fee_eviction_decision(&tx, &min_tx) {
                            crate::mempool::eviction::FeeEvictionAction::EvictAndAdmit { .. } => {
                                self.remove(&min_id);
                            }
                            crate::mempool::eviction::FeeEvictionAction::RejectNewTx => {
                                return Err(TransactionError::InsufficientFee {
                                    minimum:  self.config.min_fee,
                                    provided: tx.fee(),
                                });
                            }
                        }
                    } else {
                        return Err(TransactionError::MempoolFull);
                    }
                } else {
                    return Err(TransactionError::MempoolFull);
                }
            } else {
                return Err(TransactionError::MempoolFull);
            }
        }

        if self.by_id.contains_key(tx.tx_id()) {
            return Err(TransactionError::DuplicateTransaction);
        }

        let addr_key   = tx.from().to_checksum_hex();
        let addr_count = self.by_address.get(&addr_key).map(|s| s.len()).unwrap_or(0);
        if addr_count >= self.config.max_per_address {
            return Err(TransactionError::MempoolAddressLimitReached);
        }

        if tx.fee() < self.config.min_fee {
            return Err(TransactionError::InsufficientFee {
                minimum:  self.config.min_fee,
                provided: tx.fee(),
            });
        }

        let tx_id   = tx.tx_id().clone();
        let fee_key = FeeKey::new(&tx);
        let arc_tx  = Arc::new(tx);

        self.by_fee.insert(fee_key, tx_id.clone());
        self.by_address.entry(addr_key).or_insert_with(HashSet::new).insert(tx_id.clone());
        self.by_id.insert(tx_id, arc_tx);

        Ok(())
    }

    pub fn remove(&mut self, tx_id: &HashDigest) {
        if let Some(tx) = self.by_id.remove(tx_id) {
            let fee_key = FeeKey::new(&tx);
            self.by_fee.remove(&fee_key);
            let addr_key = tx.from().to_checksum_hex();
            if let Some(set) = self.by_address.get_mut(&addr_key) {
                set.remove(tx_id);
                if set.is_empty() { self.by_address.remove(&addr_key); }
            }
        }
    }

    pub fn remove_batch(&mut self, tx_ids: &[HashDigest]) {
        for tx_id in tx_ids { self.remove(tx_id); }
    }

    pub fn contains(&self, tx_id: &HashDigest) -> bool {
        self.by_id.contains_key(tx_id)
    }

    pub fn get(&self, tx_id: &HashDigest) -> Option<Arc<Transaction>> {
        self.by_id.get(tx_id).cloned()
    }

    pub fn pending_for(&self, addr_hex: &str) -> Vec<Arc<Transaction>> {
        match self.by_address.get(addr_hex) {
            None      => vec![],
            Some(ids) => ids.iter().filter_map(|id| self.by_id.get(id).cloned()).collect(),
        }
    }

    pub fn top_n(&self, n: usize) -> Vec<Arc<Transaction>> {
        self.by_fee.values().take(n).filter_map(|id| self.by_id.get(id).cloned()).collect()
    }

    pub fn all_ordered(&self) -> Vec<Arc<Transaction>> {
        self.by_fee.values().filter_map(|id| self.by_id.get(id).cloned()).collect()
    }

    pub fn len(&self)     -> usize { self.by_id.len() }
    pub fn is_empty(&self) -> bool { self.by_id.is_empty() }
    pub fn is_full(&self)  -> bool { self.by_id.len() >= self.config.max_size }
    pub fn config(&self)   -> &MempoolConfig { &self.config }

    pub fn evict_expired(&mut self, now: Timestamp) -> usize {
        let policy  = EvictionPolicy::new(self.config.tx_ttl_ms);
        let expired: Vec<HashDigest> = self.by_id.values()
            .filter(|tx| policy.is_expired(tx, now))
            .map(|tx| tx.tx_id().clone())
            .collect();
        let count = expired.len();
        for tx_id in &expired { self.remove(tx_id); }
        count
    }
}
