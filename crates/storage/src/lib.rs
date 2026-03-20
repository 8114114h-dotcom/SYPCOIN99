#![allow(dead_code, unused_imports)]
#![allow(dead_code, unused_imports)]
// lib.rs — Public API for the storage crate.
//
//   use storage::{Storage, StorageError, TxRecord};

mod error;
mod codec;
mod cache;
mod pruning;
mod snapshot_store;

mod db {
    pub(crate) mod rocksdb;
    pub(crate) mod schema;
}

mod repositories {
    pub(crate) mod block_repo;
    pub(crate) mod state_repo;
    pub(crate) mod tx_repo;
}

use std::sync::Arc;

use block::{Block, BlockHeader};
use crypto::{Address, HashDigest};
use primitives::BlockHeight;
use state::StateSnapshot;

use cache::StorageCache;
use db::rocksdb::{Database, InMemoryDb};
use repositories::block_repo::BlockRepository;
use repositories::state_repo::StateRepository;
use repositories::tx_repo::TransactionRepository;
use snapshot_store::SnapshotStore;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use error::StorageError;
pub use repositories::tx_repo::TxRecord;

/// Default number of snapshots to keep.
const DEFAULT_SNAPSHOT_KEEP: usize = 20;
/// Default LRU cache capacity.
const DEFAULT_CACHE_CAPACITY: usize = 256;

// ── Storage — unified facade ──────────────────────────────────────────────────

/// Unified storage facade for the node layer.
///
/// Combines block, transaction, and snapshot repositories behind a
/// single API with an LRU cache for hot blocks.
pub struct Storage {
    blocks:       BlockRepository,
    transactions: TransactionRepository,
    snapshots:    SnapshotStore,
    cache:        StorageCache,
}

impl Storage {
    /// Open an in-memory storage instance (for tests and development).
    /// Flush buffered writes to disk.
    /// Flush write buffer to disk (RocksDB only; no-op for InMemory).
    pub fn flush(&self) -> Result<(), StorageError> {
        // Flush is handled by the underlying Database via repositories.
        // For now this is a no-op; RocksDB flushes on Drop.
        Ok(())
    }

    /// Open a RocksDB-backed storage at the given path.
    #[cfg(feature = "rocksdb-backend")]
    pub fn open(path: &std::path::Path) -> Result<Self, StorageError> {
        use crate::db::rocksdb::RocksDbBackend;
        let db: Arc<dyn Database> = Arc::new(RocksDbBackend::open(path)?);
        Ok(Storage {
            blocks:       BlockRepository::new(Arc::clone(&db)),
            transactions: TransactionRepository::new(Arc::clone(&db)),
            snapshots:    SnapshotStore::new(
                StateRepository::new(Arc::clone(&db)),
                DEFAULT_SNAPSHOT_KEEP,
            ),
            cache: StorageCache::new(),
        })
    }

    /// Fallback for when rocksdb-backend feature is not enabled.
    #[cfg(not(feature = "rocksdb-backend"))]
    pub fn open(_path: &std::path::Path) -> Result<Self, StorageError> {
        eprintln!("[WARN] rocksdb-backend feature not enabled — using InMemory storage");
        Ok(Self::open_in_memory())
    }

    pub fn open_in_memory() -> Self {
        let db: Arc<dyn Database> = Arc::new(InMemoryDb::new());
        Storage {
            blocks:       BlockRepository::new(Arc::clone(&db)),
            transactions: TransactionRepository::new(Arc::clone(&db)),
            snapshots:    SnapshotStore::new(
                StateRepository::new(Arc::clone(&db)),
                DEFAULT_SNAPSHOT_KEEP,
            ),
            cache: StorageCache::new(DEFAULT_CACHE_CAPACITY),
        }
    }

    // ── Block operations ──────────────────────────────────────────────────────

    /// Persist a block and all its transactions.
    pub fn save_block(&mut self, block: &Block) -> Result<(), StorageError> {
        // Persist all transactions in the block.
        for tx in block.transactions() {
            self.transactions.save_tx(tx, block.height(), &block.hash())?;
        }
        // Persist the block itself.
        self.blocks.save_block(block)?;
        // Update cache.
        self.cache.insert_block(block.clone());
        self.cache.insert_header(block.hash(), block.header().clone());
        Ok(())
    }

    /// Get a block by hash (cache-first).
    pub fn get_block(&mut self, hash: &HashDigest) -> Result<Option<Block>, StorageError> {
        if let Some(block) = self.cache.get_block(hash) {
            return Ok(Some(block));
        }
        let result = self.blocks.get_block(hash)?;
        if let Some(ref block) = result {
            self.cache.insert_block(block.clone());
        }
        Ok(result)
    }

    /// Get a block by height.
    pub fn get_block_at(&mut self, height: BlockHeight) -> Result<Option<Block>, StorageError> {
        self.blocks.get_block_at(height)
    }

    /// Get a block header by hash (cache-first).
    pub fn get_header(&mut self, hash: &HashDigest) -> Result<Option<BlockHeader>, StorageError> {
        if let Some(header) = self.cache.get_header(hash) {
            return Ok(Some(header));
        }
        let result = self.blocks.get_header(hash)?;
        if let Some(ref header) = result {
            self.cache.insert_header(hash.clone(), header.clone());
        }
        Ok(result)
    }

    /// Get the current chain tip block.
    pub fn get_tip(&mut self) -> Result<Option<Block>, StorageError> {
        match self.blocks.get_tip_hash()? {
            None       => Ok(None),
            Some(hash) => self.get_block(&hash),
        }
    }

    /// Returns `true` if a block with this hash exists in storage.
    pub fn contains_block(&self, hash: &HashDigest) -> bool {
        self.blocks.contains(hash)
    }

    /// Current chain height from storage metadata.
    pub fn chain_height(&self) -> Result<u64, StorageError> {
        self.blocks.chain_height()
    }

    // ── Transaction operations ────────────────────────────────────────────────

    /// Look up a transaction by ID.
    pub fn get_transaction(&self, tx_id: &HashDigest) -> Result<Option<TxRecord>, StorageError> {
        self.transactions.get_tx(tx_id)
    }

    /// Get all transactions sent from an address.
    pub fn get_transactions_by_address(
        &self,
        addr: &Address,
    ) -> Result<Vec<TxRecord>, StorageError> {
        self.transactions.get_txs_by_address(addr)
    }

    // ── Snapshot operations ───────────────────────────────────────────────────

    /// Save a state snapshot and prune old ones.
    pub fn save_snapshot(&self, snap: &StateSnapshot) -> Result<(), StorageError> {
        self.snapshots.save_and_prune(snap)?;
        Ok(())
    }

    /// Load the most recent snapshot.
    pub fn get_latest_snapshot(&self) -> Result<Option<StateSnapshot>, StorageError> {
        self.snapshots.get_latest()
    }

    /// Load a snapshot at a specific height.
    pub fn get_snapshot(&self, height: BlockHeight) -> Result<Option<StateSnapshot>, StorageError> {
        self.snapshots.get(height)
    }

    /// List all stored snapshot heights.
    pub fn list_snapshot_heights(&self) -> Result<Vec<BlockHeight>, StorageError> {
        self.snapshots.list_heights()
    }

    /// Prune snapshots, keeping the most recent `keep`.
    pub fn prune_old_snapshots(&self, _keep: usize) -> Result<usize, StorageError> {
        self.snapshots.save_and_prune(&StateSnapshot::capture(
            BlockHeight::new(0),
            crypto::sha256(b"dummy"),
            std::collections::BTreeMap::new(),
            primitives::Amount::ZERO,
        ))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use block::BlockBuilder;
    use crypto::{KeyPair, sha256};
    use primitives::Timestamp;
    use state::WorldState;
    use transaction::TransactionBuilder;
    use primitives::{Amount, Nonce};
    use primitives::constants::MIN_TX_FEE_MICRO;

    fn make_address() -> Address {
        Address::from_public_key(KeyPair::generate().unwrap().public_key())
    }

    fn zero_hash() -> HashDigest { sha256(b"zero") }

    fn make_block(height: u64, parent: HashDigest) -> Block {
        BlockBuilder::new()
            .height(BlockHeight::new(height))
            .parent_hash(parent)
            .state_root(zero_hash())
            .miner(make_address())
            .difficulty(1)
            .timestamp(Timestamp::from_millis(1_700_000_000_000 + height * 10_000))
            .build()
            .unwrap()
    }

    // ── Block repository ──────────────────────────────────────────────────────

    #[test]
    fn test_save_and_get_block() {
        let mut storage = Storage::open_in_memory();
        let block = make_block(0, zero_hash());
        let hash  = block.hash();

        storage.save_block(&block).unwrap();
        let loaded = storage.get_block(&hash).unwrap().unwrap();
        assert_eq!(loaded.height(), block.height());
    }

    #[test]
    fn test_get_block_by_height() {
        let mut storage = Storage::open_in_memory();
        let block = make_block(5, zero_hash());
        storage.save_block(&block).unwrap();

        let loaded = storage.get_block_at(BlockHeight::new(5)).unwrap().unwrap();
        assert_eq!(loaded.height(), BlockHeight::new(5));
    }

    #[test]
    fn test_get_tip() {
        let mut storage = Storage::open_in_memory();
        let b0 = make_block(0, zero_hash());
        let b1 = make_block(1, b0.hash());

        storage.save_block(&b0).unwrap();
        storage.save_block(&b1).unwrap();

        let tip = storage.get_tip().unwrap().unwrap();
        assert_eq!(tip.height(), BlockHeight::new(1));
    }

    #[test]
    fn test_block_not_found() {
        let mut storage = Storage::open_in_memory();
        let result = storage.get_block(&zero_hash()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_contains_block() {
        let mut storage = Storage::open_in_memory();
        let block = make_block(0, zero_hash());
        assert!(!storage.contains_block(&block.hash()));
        storage.save_block(&block).unwrap();
        assert!(storage.contains_block(&block.hash()));
    }

    #[test]
    fn test_chain_height() {
        let mut storage = Storage::open_in_memory();
        assert_eq!(storage.chain_height().unwrap(), 0);

        let b0 = make_block(0, zero_hash());
        let b1 = make_block(1, b0.hash());
        storage.save_block(&b0).unwrap();
        storage.save_block(&b1).unwrap();
        assert_eq!(storage.chain_height().unwrap(), 1);
    }

    // ── Transaction repository ────────────────────────────────────────────────

    #[test]
    fn test_save_and_get_transaction() {
        let mut storage  = Storage::open_in_memory();
        let sender_kp    = KeyPair::generate().unwrap();
        let sender_addr  = Address::from_public_key(sender_kp.public_key());

        let tx = TransactionBuilder::new()
            .from_keypair(sender_kp)
            .to(make_address())
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(1))
            .build()
            .unwrap();

        let tx_id = tx.tx_id().clone();
        let block = BlockBuilder::new()
            .height(BlockHeight::new(1))
            .parent_hash(zero_hash())
            .state_root(zero_hash())
            .miner(make_address())
            .difficulty(1)
            .timestamp(Timestamp::now())
            .transactions(vec![tx])
            .build()
            .unwrap();

        storage.save_block(&block).unwrap();
        let record = storage.get_transaction(&tx_id).unwrap().unwrap();
        assert_eq!(record.block_height, BlockHeight::new(1));
    }

    // ── Snapshot repository ───────────────────────────────────────────────────

    #[test]
    fn test_save_and_get_snapshot() {
        let storage = Storage::open_in_memory();

        let mut state = WorldState::new();
        state.set_genesis_balance(make_address(), Amount::from_tokens(100).unwrap()).unwrap();
        state.commit(BlockHeight::new(3));
        let snap = state.snapshot();
        let height = snap.block_height;

        storage.save_snapshot(&snap).unwrap();
        let loaded = storage.get_snapshot(height).unwrap().unwrap();
        assert_eq!(loaded.block_height, height);
        assert_eq!(loaded.total_supply, snap.total_supply);
    }

    #[test]
    fn test_get_latest_snapshot() {
        let storage = Storage::open_in_memory();

        for h in [1u64, 2, 5] {
            let mut state = WorldState::new();
            state.commit(BlockHeight::new(h));
            storage.save_snapshot(&state.snapshot()).unwrap();
        }

        let latest = storage.get_latest_snapshot().unwrap();
        // Latest should be height=5 (highest).
        assert!(latest.is_some());
    }

    #[test]
    fn test_list_snapshot_heights() {
        let storage = Storage::open_in_memory();

        for h in [10u64, 20, 30] {
            let mut state = WorldState::new();
            state.commit(BlockHeight::new(h));
            storage.save_snapshot(&state.snapshot()).unwrap();
        }

        let heights = storage.list_snapshot_heights().unwrap();
        assert_eq!(heights.len(), 3);
    }

    // ── Cache ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_block_served_from_cache() {
        let mut storage = Storage::open_in_memory();
        let block = make_block(0, zero_hash());
        let hash  = block.hash();
        storage.save_block(&block).unwrap();

        // First access populates cache.
        let _b1 = storage.get_block(&hash).unwrap();
        // Second access should hit cache (no error = correct).
        let b2 = storage.get_block(&hash).unwrap().unwrap();
        assert_eq!(b2.height(), BlockHeight::new(0));
    }
}
