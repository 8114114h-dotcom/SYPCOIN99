// loader.rs — GenesisConfig definition and TOML loader.
//
// The genesis configuration is the single source of truth for chain
// parameters at block 0. It is loaded from a TOML file (config/genesis.toml)
// and must be identical on every node in the network — any difference
// produces a different genesis hash and a network split.

use serde::{Deserialize, Serialize};

use primitives::constants::{
    CHAIN_ID, CHAIN_NAME, TARGET_BLOCK_TIME_MS,
};

use crate::error::GenesisError;

/// A single account pre-funded at genesis.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenesisAccount {
    /// Checksum hex address (e.g. "0xAbCd...").
    pub address: String,
    /// Decimal token balance (e.g. "1000.000000").
    pub balance: String,
    /// Optional human-readable label (not stored on-chain).
    pub label:   Option<String>,
}

/// Full genesis configuration.
///
/// Loaded from `config/genesis.toml` before node startup.
/// Must be identical on every node for network consensus.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenesisConfig {
    /// Must match `primitives::constants::CHAIN_ID`.
    pub chain_id:           u64,

    /// Human-readable chain name (for display only, not consensus).
    pub chain_name:         String,

    /// PoW difficulty for the genesis block and initial mining.
    pub initial_difficulty: u64,

    /// Target time between blocks in milliseconds.
    pub block_time_ms:      u64,

    /// Pre-funded accounts at genesis.
    pub initial_accounts:   Vec<GenesisAccount>,

    /// Genesis block timestamp (Unix milliseconds).
    /// Must be a fixed value — using Timestamp::now() would make genesis
    /// non-deterministic across nodes.
    pub timestamp:          u64,

    /// Optional message embedded in the genesis block (Satoshi-style).
    pub message:            Option<String>,
}

impl GenesisConfig {
    /// Validate the config against compile-time constants.
    pub fn validate(&self) -> Result<(), GenesisError> {
        if self.chain_id != CHAIN_ID {
            return Err(GenesisError::InvalidChainId {
                expected: CHAIN_ID,
                got:      self.chain_id,
            });
        }
        if self.initial_difficulty == 0 {
            return Err(GenesisError::InvalidDifficulty);
        }
        Ok(())
    }
}

/// Loads and parses `GenesisConfig` from TOML.
pub struct GenesisLoader;

impl GenesisLoader {
    /// Parse genesis config from a TOML string.
    pub fn from_str(toml_str: &str) -> Result<GenesisConfig, GenesisError> {
        let config: GenesisConfig = toml::from_str(toml_str)
            .map_err(|e| GenesisError::ParseError(e.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    /// Parse genesis config from a TOML file path.
    pub fn from_file(path: &str) -> Result<GenesisConfig, GenesisError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| GenesisError::ParseError(
                format!("cannot read '{}': {}", path, e)
            ))?;
        Self::from_str(&content)
    }

    /// Default genesis config for development and testing.
    ///
    /// Uses difficulty=1 (mines instantly) and no pre-funded accounts.
    pub fn default_config() -> GenesisConfig {
        GenesisConfig {
            chain_id:           CHAIN_ID,
            chain_name:         CHAIN_NAME.to_owned(),
            initial_difficulty: 1,
            block_time_ms:      TARGET_BLOCK_TIME_MS,
            initial_accounts:   vec![],
            // Fixed timestamp for determinism in tests.
            timestamp:          1_700_000_000_000,
            message:            Some("Sypcoin genesis block".to_owned()),
        }
    }

    /// Serialize a config to TOML string (for writing genesis.toml).
    pub fn to_toml(config: &GenesisConfig) -> Result<String, GenesisError> {
        toml::to_string_pretty(config)
            .map_err(|e| GenesisError::ParseError(e.to_string()))
    }
}
