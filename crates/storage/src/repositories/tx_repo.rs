// repositories/tx_repo.rs — Transaction indexing and lookup.

use std::sync::Arc;

use crypto::{Address, HashDigest};
use primitives::BlockHeight;
use serde::{Deserialize, Serialize};
use transaction::Transaction;

use crate::codec::{deserialize, serialize};
use crate::db::rocksdb::{Database, WriteBatch};
use crate::db::schema::{tx_addr_key, tx_key, PREFIX_TX_BY_ADDR};
use crate::error::StorageError;

/// A transaction record stored on disk, linking the tx to its block.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TxRecord {
    pub tx:           Transaction,
    pub block_height: BlockHeight,
    pub block_hash:   HashDigest,
}

pub struct TransactionRepository {
    db: Arc<dyn Database>,
}

impl TransactionRepository {
    pub fn new(db: Arc<dyn Database>) -> Self {
        TransactionRepository { db }
    }

    /// Persist a transaction and index it by sender address.
    pub fn save_tx(
        &self,
        tx:     &Transaction,
        height: BlockHeight,
        bh:     &HashDigest,
    ) -> Result<(), StorageError> {
        let record = TxRecord {
            tx:           tx.clone(),
            block_height: height,
            block_hash:   bh.clone(),
        };
        let record_bytes = serialize(&record)?;
        let tx_id_bytes  = serialize(tx.tx_id())?;
        let addr_hex     = tx.from().to_checksum_hex();

        let mut batch = WriteBatch::new();
        // Primary index: tx_id → TxRecord
        batch.put(tx_key(tx.tx_id()).as_bytes(), record_bytes);
        // Secondary index: address+tx_id → tx_id (for address lookup)
        batch.put(
            tx_addr_key(&addr_hex, tx.tx_id()).as_bytes(),
            tx_id_bytes,
        );
        self.db.write_batch(batch)
    }

    pub fn get_tx(&self, tx_id: &HashDigest) -> Result<Option<TxRecord>, StorageError> {
        match self.db.get(tx_key(tx_id).as_bytes())? {
            None        => Ok(None),
            Some(bytes) => Ok(Some(deserialize(&bytes)?)),
        }
    }

    /// Return all transactions sent by `addr`, in storage order.
    pub fn get_txs_by_address(&self, addr: &Address) -> Result<Vec<TxRecord>, StorageError> {
        let prefix = format!("{}{}", PREFIX_TX_BY_ADDR, addr.to_checksum_hex());
        let entries = self.db.prefix_scan(prefix.as_bytes())?;

        let mut records = Vec::with_capacity(entries.len());
        for (_, tx_id_bytes) in entries {
            let tx_id: HashDigest = deserialize(&tx_id_bytes)?;
            if let Some(record) = self.get_tx(&tx_id)? {
                records.push(record);
            }
        }
        Ok(records)
    }
}
