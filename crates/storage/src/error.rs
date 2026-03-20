// error.rs — Unified error type for the storage crate.

use thiserror::Error;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum StorageError {
    /// Underlying database operation failed.
    #[error("database error: {0}")]
    DatabaseError(String),

    /// Failed to serialize a value to bytes.
    #[error("serialization error: {0}")]
    SerializationError(String),

    /// Failed to deserialize bytes into a value.
    #[error("deserialization error: {0}")]
    DeserializationError(String),

    /// A requested key was not found in the database.
    #[error("key not found: {0}")]
    KeyNotFound(String),

    /// Data on disk is corrupted or in an unexpected format.
    #[error("corrupted data: {0}")]
    CorruptedData(String),

    /// I/O error (file system, permissions, etc.).
    #[error("I/O error: {0}")]
    IoError(String),

    /// Snapshot at requested height does not exist.
    #[error("no snapshot at height {0}")]
    SnapshotNotFound(u64),
}
