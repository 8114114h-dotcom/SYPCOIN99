// consensus.rs — Consensus and mining configuration.

use serde::{Deserialize, Serialize};
use primitives::constants::{
    TARGET_BLOCK_TIME_MS, DIFFICULTY_ADJUSTMENT_INTERVAL,
};
use crate::error::ConfigError;

/// Consensus and mining configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Start mining automatically when the node starts.
    #[serde(default)]
    pub mine_on_start: bool,

    /// Address that receives block rewards (checksum hex).
    /// Required if mine_on_start = true.
    #[serde(default)]
    pub miner_address: Option<String>,

    /// Starting difficulty for the genesis block.
    #[serde(default = "default_difficulty")]
    pub initial_difficulty: u64,

    /// Target time between blocks in milliseconds.
    #[serde(default = "default_block_time")]
    pub target_block_time_ms: u64,

    /// Maximum chain reorganisation depth allowed.
    #[serde(default = "default_reorg_depth")]
    pub max_reorg_depth: u64,

    /// Number of blocks between difficulty adjustments.
    #[serde(default = "default_adjustment_interval")]
    pub difficulty_adjustment_interval: u64,
}

impl ConsensusConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.initial_difficulty == 0 {
            return Err(ConfigError::InvalidValue {
                field:  "initial_difficulty".into(),
                reason: "must be ≥ 1".into(),
            });
        }
        if self.mine_on_start && self.miner_address.is_none() {
            return Err(ConfigError::InvalidValue {
                field:  "miner_address".into(),
                reason: "required when mine_on_start = true".into(),
            });
        }
        if self.target_block_time_ms == 0 {
            return Err(ConfigError::InvalidValue {
                field:  "target_block_time_ms".into(),
                reason: "must be > 0".into(),
            });
        }
        Ok(())
    }
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        ConsensusConfig {
            mine_on_start:                  false,
            miner_address:                  None,
            initial_difficulty:             default_difficulty(),
            target_block_time_ms:           default_block_time(),
            max_reorg_depth:                default_reorg_depth(),
            difficulty_adjustment_interval: default_adjustment_interval(),
        }
    }
}

fn default_difficulty()           -> u64 { 1_000 }
fn default_block_time()           -> u64 { TARGET_BLOCK_TIME_MS }
fn default_reorg_depth()          -> u64 { 100 }
fn default_adjustment_interval()  -> u64 { DIFFICULTY_ADJUSTMENT_INTERVAL }
