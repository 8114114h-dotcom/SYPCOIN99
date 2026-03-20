// storage.rs — Storage and database configuration.

use serde::{Deserialize, Serialize};
use crate::error::ConfigError;

/// Which database backend to use.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DbBackend {
    /// In-memory only — data is lost on restart. For development/tests.
    InMemory,
    /// RocksDB — persistent storage. For production.
    RocksDb,
}

/// Storage and persistence configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Root directory for all on-disk data.
    #[serde(default = "default_data_dir")]
    pub data_dir: String,

    /// Database backend.
    #[serde(default = "default_db_backend")]
    pub db_backend: DbBackend,

    /// Save a state snapshot every N blocks.
    #[serde(default = "default_snapshot_interval")]
    pub snapshot_interval: u64,

    /// Number of snapshots to keep (older ones are pruned).
    #[serde(default = "default_snapshots_to_keep")]
    pub snapshots_to_keep: usize,

    /// Maximum database size in megabytes (0 = unlimited).
    #[serde(default)]
    pub max_db_size_mb: u64,
}

impl StorageConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.snapshot_interval == 0 {
            return Err(ConfigError::InvalidValue {
                field:  "snapshot_interval".into(),
                reason: "must be ≥ 1".into(),
            });
        }
        if self.snapshots_to_keep == 0 {
            return Err(ConfigError::InvalidValue {
                field:  "snapshots_to_keep".into(),
                reason: "must be ≥ 1".into(),
            });
        }
        if self.db_backend == DbBackend::RocksDb && self.data_dir.is_empty() {
            return Err(ConfigError::InvalidValue {
                field:  "data_dir".into(),
                reason: "required for RocksDB backend".into(),
            });
        }
        Ok(())
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        StorageConfig {
            data_dir:          default_data_dir(),
            db_backend:        default_db_backend(),
            snapshot_interval: default_snapshot_interval(),
            snapshots_to_keep: default_snapshots_to_keep(),
            max_db_size_mb:    0,
        }
    }
}

fn default_data_dir()          -> String    { "./data".into() }
fn default_db_backend()        -> DbBackend { DbBackend::RocksDb }
fn default_snapshot_interval() -> u64       { 1_000 }
fn default_snapshots_to_keep() -> usize     { 20 }
