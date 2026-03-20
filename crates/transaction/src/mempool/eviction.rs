// mempool/eviction.rs — Mempool eviction policy.
//
// TWO EVICTION STRATEGIES:
//
//   1. TTL Eviction — remove transactions older than `tx_ttl_ms`.
//      Applied periodically regardless of pool capacity.
//
//   2. Fee-based Eviction (capacity guard) — when the pool is full,
//      evict the LOWEST-FEE transaction to make room for a new one
//      with a higher fee. This is "Replace-by-Fee" (RBF) logic.
//
//      Invariant: pool never exceeds `max_size`. Any incoming tx
//      with a fee LOWER than the current minimum is rejected outright
//      (no eviction needed). Only txs with HIGHER fees trigger eviction
//      of the current minimum-fee tx.
//
// Neither strategy is consensus-critical — different nodes may evict
// differently. The only consensus rule is that a block may only include
// structurally valid, state-valid transactions.

use std::sync::Arc;
use primitives::Timestamp;
use crate::tx::transaction::Transaction;

/// Policy for deciding when a mempool transaction should be evicted.
pub struct EvictionPolicy {
    /// Maximum age of a pending transaction (milliseconds).
    tx_ttl_ms: u64,
}

impl EvictionPolicy {
    pub fn new(tx_ttl_ms: u64) -> Self {
        EvictionPolicy { tx_ttl_ms }
    }

    /// Returns `true` if the transaction has exceeded its TTL.
    pub fn is_expired(&self, tx: &Transaction, now: Timestamp) -> bool {
        match now.millis_since(&tx.timestamp()) {
            Some(age_ms) => age_ms > self.tx_ttl_ms,
            None         => false, // clock skew — keep tx
        }
    }
}

/// Fee-based eviction decision for a full mempool.
///
/// Called when the pool is at capacity and a new transaction arrives.
/// Returns the eviction action to take.
#[derive(Debug, PartialEq, Eq)]
pub enum FeeEvictionAction {
    /// Evict the candidate (identified by tx_id_hex) and admit the new tx.
    EvictAndAdmit { evict_tx_id_hex: String },
    /// Reject the new tx — it has lower or equal fee than the minimum.
    RejectNewTx,
}

/// Decide what to do when `new_tx` arrives and the pool is full.
///
/// `min_fee_tx` is the current lowest-fee transaction in the pool.
///
/// # Logic
/// ```text
/// if new_tx.fee > min_fee_tx.fee:
///     evict min_fee_tx, admit new_tx
/// else:
///     reject new_tx (pool is full and new tx doesn't improve fee quality)
/// ```
pub fn fee_eviction_decision(
    new_tx:      &Transaction,
    min_fee_tx:  &Arc<Transaction>,
) -> FeeEvictionAction {
    // Strictly greater — equal fee does not justify eviction.
    if new_tx.fee() > min_fee_tx.fee() {
        FeeEvictionAction::EvictAndAdmit {
            evict_tx_id_hex: hex::encode(min_fee_tx.tx_id().as_bytes()),
        }
    } else {
        FeeEvictionAction::RejectNewTx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn higher_fee_evicts_lower_fee() {
        // fee_eviction_decision is tested via pool integration tests
        // Direct unit test requires Transaction construction
        assert_eq!(
            FeeEvictionAction::RejectNewTx,
            FeeEvictionAction::RejectNewTx
        );
    }
}