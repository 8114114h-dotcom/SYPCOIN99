// anti_spam.rs — Anti-spam protection for transactions.
//
// Three layers:
//   1. Fee floor  — rejects txs below MIN_TX_FEE_MICRO.
//   2. Size limit — rejects txs exceeding MAX_TX_SIZE_BYTES.
//   3. Blacklist  — rejects txs from permanently blacklisted addresses.
//   4. Block spam — limits how many txs one address can have per block.

use std::collections::HashMap;
use std::collections::HashSet;

use crypto::Address;
use primitives::constants::{MIN_TX_FEE_MICRO};
use transaction::Transaction;

use crate::error::SecurityError;

/// Maximum transactions from a single address allowed per block.
pub const MAX_TX_PER_ADDRESS_PER_BLOCK: u32 = 10;

/// Maximum serialised transaction size in bytes.
pub const MAX_TX_SIZE_BYTES: usize = 512;

/// Configuration for the anti-spam filter.
pub struct AntiSpamConfig {
    pub min_fee_micro:                u64,
    pub max_tx_size:                  usize,
    pub max_tx_per_address_per_block: u32,
}

impl Default for AntiSpamConfig {
    fn default() -> Self {
        AntiSpamConfig {
            min_fee_micro:                MIN_TX_FEE_MICRO,
            max_tx_size:                  MAX_TX_SIZE_BYTES,
            max_tx_per_address_per_block: MAX_TX_PER_ADDRESS_PER_BLOCK,
        }
    }
}

/// Anti-spam filter — checks individual transactions and block-level limits.
pub struct AntiSpam {
    config:    AntiSpamConfig,
    blacklist: HashSet<String>,   // checksum-hex addresses
}

impl AntiSpam {
    pub fn new(config: AntiSpamConfig) -> Self {
        AntiSpam { config, blacklist: HashSet::new() }
    }

    pub fn with_defaults() -> Self {
        Self::new(AntiSpamConfig::default())
    }

    // ── Per-transaction checks ────────────────────────────────────────────────

    /// Run all per-transaction spam checks.
    pub fn check_transaction(&self, tx: &Transaction) -> Result<(), SecurityError> {
        let addr = tx.from().to_checksum_hex();

        // 1. Blacklist check.
        if self.blacklist.contains(&addr) {
            return Err(SecurityError::BlacklistedAddress { address: addr });
        }

        // 2. Fee floor.
        if tx.fee().as_micro() < self.config.min_fee_micro {
            return Err(SecurityError::FeeTooLow {
                min: self.config.min_fee_micro,
                got: tx.fee().as_micro(),
            });
        }

        // 3. Size limit.
        let size = tx.size_bytes();
        if size > self.config.max_tx_size {
            return Err(SecurityError::TxTooLarge {
                max: self.config.max_tx_size,
                got: size,
            });
        }

        Ok(())
    }

    // ── Block-level checks ────────────────────────────────────────────────────

    /// Verify that no single address exceeds the per-block tx limit.
    ///
    /// Call this before including a list of transactions in a block.
    pub fn check_block_tx_distribution(
        &self,
        txs: &[Transaction],
    ) -> Result<(), SecurityError> {
        let mut counts: HashMap<String, u32> = HashMap::new();

        for tx in txs {
            let addr = tx.from().to_checksum_hex();
            let count = counts.entry(addr.clone()).or_insert(0);
            *count += 1;

            if *count > self.config.max_tx_per_address_per_block {
                return Err(SecurityError::BlockSpamDetected {
                    address: addr,
                    count:   *count,
                    max:     self.config.max_tx_per_address_per_block,
                });
            }
        }
        Ok(())
    }

    // ── Blacklist management ──────────────────────────────────────────────────

    /// Add an address to the permanent blacklist.
    pub fn blacklist_address(&mut self, addr: &Address) {
        self.blacklist.insert(addr.to_checksum_hex());
    }

    /// Remove an address from the blacklist.
    pub fn unblacklist_address(&mut self, addr: &Address) {
        self.blacklist.remove(&addr.to_checksum_hex());
    }

    pub fn is_blacklisted(&self, addr: &Address) -> bool {
        self.blacklist.contains(&addr.to_checksum_hex())
    }

    pub fn blacklist_size(&self) -> usize {
        self.blacklist.len()
    }
}
