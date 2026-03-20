// db/rocksdb.rs — Database backend abstraction.
//
// We define a `Database` trait and two implementations:
//
//   InMemoryDb  — HashMap-backed, zero dependencies, used for tests and CI.
//   RocksDbBackend — RocksDB-backed, enabled via `rocksdb-backend` feature.
//
// The rest of the crate only uses the `Database` trait, so swapping backends
// requires no changes outside this file.
//
// Design:
//   • Simple key-value interface: get / put / delete / prefix_scan.
//   • WriteBatch for atomic multi-key writes.
//   • All keys and values are raw bytes (Vec<u8>).
//   • Serialization/deserialization is the responsibility of repositories.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use crate::error::StorageError;

// ── Database trait ────────────────────────────────────────────────────────────

/// Minimal key-value database interface.
pub trait Database: Send + Sync {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError>;
    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), StorageError>;
    fn delete(&self, key: &[u8]) -> Result<(), StorageError>;

    /// Return all (key, value) pairs whose key starts with `prefix`.
    /// Keys are returned in lexicographic order.
    fn prefix_scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>, StorageError>;

    /// Atomically apply a batch of writes.
    fn write_batch(&self, batch: WriteBatch) -> Result<(), StorageError>;

    /// Flush buffered writes to disk (no-op for InMemory).
    fn flush(&self) -> Result<(), StorageError> { Ok(()) }
}

/// A batch of atomic writes.
#[derive(Default)]
pub struct WriteBatch {
    pub ops: Vec<BatchOp>,
}

pub enum BatchOp {
    Put { key: Vec<u8>, value: Vec<u8> },
    Delete { key: Vec<u8> },
}

impl WriteBatch {
    pub fn new() -> Self { WriteBatch::default() }

    pub fn put(&mut self, key: impl Into<Vec<u8>>, value: impl Into<Vec<u8>>) {
        self.ops.push(BatchOp::Put { key: key.into(), value: value.into() });
    }

    pub fn delete(&mut self, key: impl Into<Vec<u8>>) {
        self.ops.push(BatchOp::Delete { key: key.into() });
    }
}

// ── In-memory backend ─────────────────────────────────────────────────────────

/// Thread-safe in-memory database backed by a BTreeMap.
/// BTreeMap provides lexicographic ordering, enabling prefix_scan.
pub struct InMemoryDb {
    data: Arc<RwLock<BTreeMap<Vec<u8>, Vec<u8>>>>,
}

impl InMemoryDb {
    pub fn new() -> Self {
        InMemoryDb {
            data: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }
}

impl Default for InMemoryDb {
    fn default() -> Self { Self::new() }
}

impl Database for InMemoryDb {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        // Use read lock — multiple concurrent reads allowed
        let data = self.data.read()
            .map_err(|e| StorageError::DatabaseError(format!("read lock poisoned: {}", e)))?;
        Ok(data.get(key).cloned())
    }

    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), StorageError> {
        let mut data = self.data.write()
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;
        data.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    fn delete(&self, key: &[u8]) -> Result<(), StorageError> {
        let mut data = self.data.write()
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;
        data.remove(key);
        Ok(())
    }

    fn prefix_scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>, StorageError> {
        let data = self.data.read()
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;
        let results = data
            .range(prefix.to_vec()..)
            .take_while(|(k, _)| k.starts_with(prefix))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Ok(results)
    }

    fn write_batch(&self, batch: WriteBatch) -> Result<(), StorageError> {
        let mut data = self.data.write()
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;
        for op in batch.ops {
            match op {
                BatchOp::Put { key, value } => { data.insert(key, value); }
                BatchOp::Delete { key }     => { data.remove(&key); }
            }
        }
        Ok(())
    }
}

// ── RocksDB backend (feature-gated) ──────────────────────────────────────────

#[cfg(feature = "rocksdb-backend")]
pub struct RocksDbBackend {
    db: rocksdb::DB,
}

#[cfg(feature = "rocksdb-backend")]
impl RocksDbBackend {
    pub fn open(path: &std::path::Path) -> Result<Self, StorageError> {
        let mut opts = rocksdb::Options::default();
        opts.create_if_missing(true);
        opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
        // Production tuning
        opts.set_write_buffer_size(64 * 1024 * 1024);      // 64MB write buffer
        opts.set_max_write_buffer_number(3);                // 3 write buffers
        opts.set_target_file_size_base(64 * 1024 * 1024);  // 64MB SST files
        opts.increase_parallelism(4);                       // 4 threads for compaction
        opts.set_bloom_locality(1);                         // bloom filter for point lookups
        opts.set_bytes_per_sync(1024 * 1024);              // 1MB sync interval

        let db = rocksdb::DB::open(&opts, path)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))?;

        Ok(RocksDbBackend { db })
    }
}

#[cfg(feature = "rocksdb-backend")]
impl Database for RocksDbBackend {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        self.db.get(key)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))
    }

    fn put(&self, key: &[u8], value: &[u8]) -> Result<(), StorageError> {
        self.db.put(key, value)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))
    }

    fn delete(&self, key: &[u8]) -> Result<(), StorageError> {
        self.db.delete(key)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))
    }

    fn prefix_scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>, StorageError> {
        let iter = self.db.prefix_iterator(prefix);
        let mut results = Vec::new();
        for item in iter {
            let (k, v) = item.map_err(|e| StorageError::DatabaseError(e.to_string()))?;
            if !k.starts_with(prefix) { break; }
            results.push((k.to_vec(), v.to_vec()));
        }
        Ok(results)
    }

    fn flush(&self) -> Result<(), StorageError> {
        self.db.flush()
            .map_err(|e| StorageError::DatabaseError(e.to_string()))
    }

    fn write_batch(&self, batch: WriteBatch) -> Result<(), StorageError> {
        let mut wb = rocksdb::WriteBatch::default();
        for op in batch.ops {
            match op {
                BatchOp::Put { key, value } => wb.put(key, value),
                BatchOp::Delete { key }     => wb.delete(key),
            }
        }
        self.db.write(wb)
            .map_err(|e| StorageError::DatabaseError(e.to_string()))
    }
}
