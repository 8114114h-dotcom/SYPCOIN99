// softfork.rs — Soft fork schedule and activation logic.
//
// A soft fork tightens existing rules — old nodes still accept new blocks
// but new nodes reject blocks that old nodes would accept.
// Soft forks are backward-compatible for non-upgraded nodes.
//
// Example: reducing max block size is a soft fork.
//          increasing it is a hard fork.

use serde::{Deserialize, Serialize};
use primitives::BlockHeight;

/// A single soft fork definition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SoftFork {
    pub id:                u32,
    pub name:              String,
    pub activation_height: BlockHeight,
    pub description:       String,
}

impl SoftFork {
    pub fn is_active_at(&self, height: BlockHeight) -> bool {
        height.as_u64() >= self.activation_height.as_u64()
    }
}

/// The complete soft fork schedule.
#[derive(Clone, Debug)]
pub struct SoftForkSchedule {
    forks: Vec<SoftFork>,
}

impl SoftForkSchedule {
    /// Official mainnet soft fork schedule.
    pub fn mainnet() -> Self {
        SoftForkSchedule {
            forks: vec![
                // No soft forks yet at launch.
                // Example:
                // SoftFork {
                //     id:                1,
                //     name:              "SegWit".into(),
                //     activation_height: BlockHeight::new(500_000),
                //     description:       "Segregated witness data".into(),
                // },
            ],
        }
    }

    pub fn is_active(&self, fork_id: u32, height: BlockHeight) -> bool {
        self.forks
            .iter()
            .find(|f| f.id == fork_id)
            .map(|f| f.is_active_at(height))
            .unwrap_or(false)
    }

    pub fn active_at(&self, height: BlockHeight) -> Vec<&SoftFork> {
        self.forks.iter().filter(|f| f.is_active_at(height)).collect()
    }

    pub fn next_fork(&self, height: BlockHeight) -> Option<&SoftFork> {
        self.forks
            .iter()
            .filter(|f| f.activation_height.as_u64() > height.as_u64())
            .min_by_key(|f| f.activation_height.as_u64())
    }
}
