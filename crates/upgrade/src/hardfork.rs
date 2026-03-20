// hardfork.rs — Hard fork schedule and activation logic.
//
// A hard fork is a non-backward-compatible protocol change.
// Nodes that do not upgrade will reject blocks from the new chain.
//
// CONSENSUS CRITICAL:
//   • Activation heights are fixed at compile time and must be identical
//     on every node. Changing them after mainnet launch is a fork.
//   • HardForkSchedule::mainnet() is the single source of truth.
//   • New forks are always added to the END of the schedule.
//     Never modify existing fork activation heights.

use serde::{Deserialize, Serialize};
use primitives::BlockHeight;

/// A single hard fork definition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HardFork {
    /// Numeric identifier (monotonically increasing).
    pub id:                u32,
    /// Short name (e.g. "Byzantium", "Shanghai").
    pub name:              String,
    /// Block height at which this fork activates.
    pub activation_height: BlockHeight,
    /// Human-readable description of the changes.
    pub description:       String,
}

impl HardFork {
    /// Returns `true` if this fork is active at the given height.
    pub fn is_active_at(&self, height: BlockHeight) -> bool {
        height.as_u64() >= self.activation_height.as_u64()
    }
}

/// The complete hard fork schedule for this network.
#[derive(Clone, Debug)]
pub struct HardForkSchedule {
    forks: Vec<HardFork>,
}

impl HardForkSchedule {
    /// Official mainnet hard fork schedule.
    ///
    /// CONSENSUS CRITICAL — never change existing entries after launch.
    pub fn mainnet() -> Self {
        HardForkSchedule {
            forks: vec![
                HardFork {
                    id:                1,
                    name:              "Genesis".into(),
                    activation_height: BlockHeight::new(0),
                    description:       "Initial protocol — genesis block rules".into(),
                },
                // Future forks are added here.
                // Example (commented out until scheduled):
                // HardFork {
                //     id:                2,
                //     name:              "Phoenix".into(),
                //     activation_height: BlockHeight::new(1_000_000),
                //     description:       "Increases max block size to 2MB".into(),
                // },
            ],
        }
    }

    /// Returns `true` if the fork with `fork_id` is active at `height`.
    pub fn is_active(&self, fork_id: u32, height: BlockHeight) -> bool {
        self.forks
            .iter()
            .find(|f| f.id == fork_id)
            .map(|f| f.is_active_at(height))
            .unwrap_or(false)
    }

    /// All forks active at the given height, in activation order.
    pub fn active_at(&self, height: BlockHeight) -> Vec<&HardFork> {
        self.forks
            .iter()
            .filter(|f| f.is_active_at(height))
            .collect()
    }

    /// The next scheduled hard fork after `height`, if any.
    pub fn next_fork(&self, height: BlockHeight) -> Option<&HardFork> {
        self.forks
            .iter()
            .filter(|f| f.activation_height.as_u64() > height.as_u64())
            .min_by_key(|f| f.activation_height.as_u64())
    }

    /// Returns `true` if `height` is exactly a hard fork activation block.
    pub fn is_fork_block(&self, height: BlockHeight) -> bool {
        self.forks
            .iter()
            .any(|f| f.activation_height.as_u64() == height.as_u64() && f.id > 1)
    }

    /// Number of blocks until the next fork (returns None if no upcoming fork).
    pub fn blocks_until_next(&self, height: BlockHeight) -> Option<u64> {
        self.next_fork(height)
            .map(|f| f.activation_height.as_u64().saturating_sub(height.as_u64()))
    }
}
