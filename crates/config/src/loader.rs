// loader.rs — NodeConfig (all settings) and ConfigLoader.

use serde::{Deserialize, Serialize};
use primitives::constants::DEFAULT_RPC_PORT;

use crate::consensus::ConsensusConfig;
use crate::error::ConfigError;
use crate::network::NetworkConfig;
use crate::storage::StorageConfig;

// ── RpcConfig ─────────────────────────────────────────────────────────────────

/// JSON-RPC server configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RpcConfig {
    /// Enable the RPC server.
    #[serde(default = "default_rpc_enabled")]
    pub enabled: bool,

    /// Address and port to listen on.
    #[serde(default = "default_rpc_addr")]
    pub listen_addr: String,

    /// Maximum simultaneous connections.
    #[serde(default = "default_max_conn")]
    pub max_connections: usize,

    /// Allowed CORS origins (empty = all origins allowed).
    #[serde(default)]
    pub cors_origins: Vec<String>,
}

impl RpcConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.enabled && self.listen_addr.is_empty() {
            return Err(ConfigError::InvalidValue {
                field:  "rpc.listen_addr".into(),
                reason: "required when rpc.enabled = true".into(),
            });
        }
        Ok(())
    }
}

impl Default for RpcConfig {
    fn default() -> Self {
        RpcConfig {
            enabled:         default_rpc_enabled(),
            listen_addr:     default_rpc_addr(),
            max_connections: default_max_conn(),
            cors_origins:    vec![],
        }
    }
}

fn default_rpc_enabled() -> bool   { true }
fn default_rpc_addr()    -> String { format!("0.0.0.0:{}", DEFAULT_RPC_PORT) }
fn default_max_conn()    -> usize  { 100 }

// ── NodeConfig ────────────────────────────────────────────────────────────────

/// Complete node configuration — aggregates all subsystem configs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeConfig {
    #[serde(default)]
    pub network:   NetworkConfig,

    #[serde(default)]
    pub consensus: ConsensusConfig,

    #[serde(default)]
    pub storage:   StorageConfig,

    #[serde(default)]
    pub rpc:       RpcConfig,

    /// Logging level: "error" | "warn" | "info" | "debug" | "trace"
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl NodeConfig {
    /// Validate all subsections.
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.network.validate()?;
        self.consensus.validate()?;
        self.storage.validate()?;
        self.rpc.validate()?;
        validate_log_level(&self.log_level)?;
        Ok(())
    }
}

impl Default for NodeConfig {
    fn default() -> Self {
        NodeConfig {
            network:   NetworkConfig::default(),
            consensus: ConsensusConfig::default(),
            storage:   StorageConfig::default(),
            rpc:       RpcConfig::default(),
            log_level: default_log_level(),
        }
    }
}

fn default_log_level() -> String { "info".into() }

fn validate_log_level(level: &str) -> Result<(), ConfigError> {
    match level {
        "error" | "warn" | "info" | "debug" | "trace" => Ok(()),
        _ => Err(ConfigError::InvalidValue {
            field:  "log_level".into(),
            reason: format!("'{}' is not valid; use error/warn/info/debug/trace", level),
        }),
    }
}

// ── ConfigLoader ──────────────────────────────────────────────────────────────

pub struct ConfigLoader;

impl ConfigLoader {
    /// Parse a NodeConfig from a TOML string.
    pub fn from_str(toml_str: &str) -> Result<NodeConfig, ConfigError> {
        let config: NodeConfig = toml::from_str(toml_str)
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    /// Load a NodeConfig from a TOML file.
    pub fn from_file(path: &str) -> Result<NodeConfig, ConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::IoError(
                format!("cannot read '{}': {}", path, e)
            ))?;
        Self::from_str(&content)
    }

    /// Default config for mainnet (no bootstrap peers, RocksDB, mine_on_start=false).
    pub fn default_mainnet() -> NodeConfig {
        NodeConfig::default()
    }

    /// Default config for testnet (easy difficulty, InMemory optional).
    pub fn default_testnet() -> NodeConfig {
        let mut cfg = NodeConfig::default();
        cfg.consensus.initial_difficulty = 100;
        cfg.network.bootstrap_peers = vec![
            "testnet-seed.sypcoin.network:30303".into(),
        ];
        cfg
    }

    /// Default config for local development (difficulty=1, InMemory, mine_on_start possible).
    pub fn default_devnet() -> NodeConfig {
        use crate::storage::DbBackend;
        let mut cfg = NodeConfig::default();
        cfg.consensus.initial_difficulty = 1;
        cfg.storage.db_backend           = DbBackend::RocksDb;
        cfg.log_level                    = "debug".into();
        cfg
    }

    /// Override config values from environment variables.
    ///
    /// Supported env vars:
    ///   SYPCOIN_LISTEN_ADDR       → network.listen_addr
    ///   SYPCOIN_RPC_ADDR          → rpc.listen_addr
    ///   SYPCOIN_DATA_DIR          → storage.data_dir
    ///   SYPCOIN_LOG_LEVEL         → log_level
    ///   SYPCOIN_MINER_ADDRESS     → consensus.miner_address
    ///   SYPCOIN_MINE_ON_START     → consensus.mine_on_start (true/false)
    pub fn apply_env(config: &mut NodeConfig) {
        if let Ok(v) = std::env::var("SYPCOIN_LISTEN_ADDR") {
            config.network.listen_addr = v;
        }
        if let Ok(v) = std::env::var("SYPCOIN_RPC_ADDR") {
            config.rpc.listen_addr = v;
        }
        if let Ok(v) = std::env::var("SYPCOIN_DATA_DIR") {
            config.storage.data_dir = v;
        }
        if let Ok(v) = std::env::var("SYPCOIN_LOG_LEVEL") {
            config.log_level = v;
        }
        if let Ok(v) = std::env::var("SYPCOIN_MINER_ADDRESS") {
            config.consensus.miner_address = Some(v);
        }
        if let Ok(v) = std::env::var("SYPCOIN_MINE_ON_START") {
            config.consensus.mine_on_start = v.to_lowercase() == "true";
        }
    }

    /// Serialize a NodeConfig to a TOML string (for writing config file).
    pub fn to_toml(config: &NodeConfig) -> Result<String, ConfigError> {
        toml::to_string_pretty(config)
            .map_err(|e| ConfigError::ParseError(e.to_string()))
    }
}
