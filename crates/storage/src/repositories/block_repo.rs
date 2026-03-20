// repositories/block_repo.rs — Block and header persistence.
//
// Every save is atomic: block, header, and height-index written in one WriteBatch.

use std::sync::Arc;

use block::{Block, BlockHeader};
use crypto::HashDigest;
use primitives::BlockHeight;

use crate::codec::{deserialize, serialize};
use crate::db::rocksdb::{Database, WriteBatch};
use crate::db::schema::{
    block_key, header_key, height_index_key,
    META_CHAIN_HEIGHT, META_TIP_HASH,
};
use crate::error::StorageError;

pub struct BlockRepository {
    db: Arc<dyn Database>,
}

impl BlockRepository {
    pub fn new(db: Arc<dyn Database>) -> Self {
        BlockRepository { db }
    }

    pub fn save_block(&self, block: &Block) -> Result<(), StorageError> {
        let block_bytes  = serialize(block)?;
        let header_bytes = serialize(block.header())?;
        let hash_bytes   = serialize(&block.hash())?;
        let height_le    = block.height().as_u64().to_le_bytes().to_vec();

        let mut batch = WriteBatch::new();
        batch.put(block_key(&block.hash()).as_bytes(),   block_bytes);
        batch.put(header_key(&block.hash()).as_bytes(),  header_bytes);
        batch.put(height_index_key(block.height()),      hash_bytes.clone());
        batch.put(META_TIP_HASH.as_bytes(),              hash_bytes);
        batch.put(META_CHAIN_HEIGHT.as_bytes(),          height_le);
        self.db.write_batch(batch)
    }

    pub fn get_block(&self, hash: &HashDigest) -> Result<Option<Block>, StorageError> {
        match self.db.get(block_key(hash).as_bytes())? {
            None        => Ok(None),
            Some(bytes) => Ok(Some(deserialize(&bytes)?)),
        }
    }

    pub fn get_block_at(&self, height: BlockHeight) -> Result<Option<Block>, StorageError> {
        match self.db.get(&height_index_key(height))? {
            None        => Ok(None),
            Some(bytes) => {
                let hash = deserialize::<HashDigest>(&bytes)?;
                self.get_block(&hash)
            }
        }
    }

    pub fn get_header(&self, hash: &HashDigest) -> Result<Option<BlockHeader>, StorageError> {
        match self.db.get(header_key(hash).as_bytes())? {
            None        => Ok(None),
            Some(bytes) => Ok(Some(deserialize(&bytes)?)),
        }
    }

    pub fn get_header_at(&self, height: BlockHeight) -> Result<Option<BlockHeader>, StorageError> {
        match self.db.get(&height_index_key(height))? {
            None        => Ok(None),
            Some(bytes) => {
                let hash = deserialize::<HashDigest>(&bytes)?;
                self.get_header(&hash)
            }
        }
    }

    pub fn get_tip_hash(&self) -> Result<Option<HashDigest>, StorageError> {
        match self.db.get(META_TIP_HASH.as_bytes())? {
            None        => Ok(None),
            Some(bytes) => Ok(Some(deserialize(&bytes)?)),
        }
    }

    pub fn chain_height(&self) -> Result<u64, StorageError> {
        match self.db.get(META_CHAIN_HEIGHT.as_bytes())? {
            None        => Ok(0),
            Some(bytes) => {
                if bytes.len() < 8 {
                    return Err(StorageError::CorruptedData("chain_height truncated".into()));
                }
                let mut arr = [0u8; 8];
                arr.copy_from_slice(&bytes[..8]);
                Ok(u64::from_le_bytes(arr))
            }
        }
    }

    pub fn contains(&self, hash: &HashDigest) -> bool {
        self.db.get(block_key(hash).as_bytes())
            .map(|v| v.is_some())
            .unwrap_or(false)
    }
}
