// repositories/state_repo.rs — StateSnapshot persistence.

use std::sync::Arc;

use primitives::BlockHeight;
use state::StateSnapshot;

use crate::codec::{deserialize, serialize};
use crate::db::rocksdb::{Database, WriteBatch};
use crate::db::schema::{snapshot_key, PREFIX_SNAPSHOT};
use crate::error::StorageError;

pub struct StateRepository {
    db: Arc<dyn Database>,
}

impl StateRepository {
    pub fn new(db: Arc<dyn Database>) -> Self {
        StateRepository { db }
    }

    pub fn save_snapshot(&self, snap: &StateSnapshot) -> Result<(), StorageError> {
        let bytes = serialize(snap)?;
        let key   = snapshot_key(snap.block_height);
        let mut batch = WriteBatch::new();
        batch.put(key, bytes);
        self.db.write_batch(batch)
    }

    pub fn get_snapshot(&self, height: BlockHeight) -> Result<Option<StateSnapshot>, StorageError> {
        match self.db.get(&snapshot_key(height))? {
            None        => Ok(None),
            Some(bytes) => Ok(Some(deserialize(&bytes)?)),
        }
    }

    /// Return the snapshot with the highest height.
    pub fn get_latest_snapshot(&self) -> Result<Option<StateSnapshot>, StorageError> {
        let entries = self.db.prefix_scan(PREFIX_SNAPSHOT.as_bytes())?;
        // prefix_scan returns keys in lexicographic order.
        // Since height is encoded as LE bytes, the last entry = highest height.
        match entries.last() {
            None             => Ok(None),
            Some((_, bytes)) => Ok(Some(deserialize(bytes)?)),
        }
    }

    /// List all stored snapshot heights in ascending order.
    pub fn list_snapshot_heights(&self) -> Result<Vec<BlockHeight>, StorageError> {
        let entries = self.db.prefix_scan(PREFIX_SNAPSHOT.as_bytes())?;
        let prefix_len = PREFIX_SNAPSHOT.len();
        let mut heights = Vec::with_capacity(entries.len());
        for (key, _) in entries {
            if key.len() >= prefix_len + 8 {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(&key[prefix_len..prefix_len + 8]);
                heights.push(BlockHeight::new(u64::from_le_bytes(arr)));
            }
        }
        Ok(heights)
    }

    pub fn delete_snapshot(&self, height: BlockHeight) -> Result<(), StorageError> {
        let mut batch = WriteBatch::new();
        batch.delete(snapshot_key(height));
        self.db.write_batch(batch)
    }
}
