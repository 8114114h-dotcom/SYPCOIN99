// lib.rs — Public API for the upgrade crate.
//
//   use upgrade::{UpgradeManager, BlockRules};

mod hardfork;
mod softfork;

pub use hardfork::{HardFork, HardForkSchedule};
pub use softfork::{SoftFork, SoftForkSchedule};

use primitives::BlockHeight;
use primitives::constants::{MAX_BLOCK_SIZE, MAX_TX_PER_BLOCK};

/// Block validation rules that may change after a hard fork.
#[derive(Clone, Debug)]
pub struct BlockRules {
    pub max_block_size:   u32,
    pub max_tx_per_block: u32,
}

impl BlockRules {
    /// The baseline rules (genesis / fork 1).
    pub fn genesis() -> Self {
        BlockRules {
            max_block_size:   MAX_BLOCK_SIZE,
            max_tx_per_block: MAX_TX_PER_BLOCK,
        }
    }
}

/// Information about an upcoming upgrade.
#[derive(Clone, Debug)]
pub struct UpgradeInfo {
    pub fork_type:         ForkType,
    pub name:              String,
    pub activation_height: BlockHeight,
    pub blocks_remaining:  u64,
    pub description:       String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ForkType { Hard, Soft }

/// Central upgrade manager — queried by the block validator and node.
pub struct UpgradeManager {
    hard_forks: HardForkSchedule,
    soft_forks: SoftForkSchedule,
}

impl UpgradeManager {
    /// Create with the mainnet schedules.
    pub fn new() -> Self {
        UpgradeManager {
            hard_forks: HardForkSchedule::mainnet(),
            soft_forks: SoftForkSchedule::mainnet(),
        }
    }

    /// Compute the block validation rules applicable at `height`.
    pub fn block_rules_at(&self, height: BlockHeight) -> BlockRules {
        // Start from genesis rules; apply any hard fork rule changes.
        let mut rules = BlockRules::genesis();

        // Future: when "Phoenix" hard fork activates at height 1_000_000,
        // rules.max_block_size = 2 * 1024 * 1024;

        rules
    }

    /// Returns `true` if the block at `height` is a hard fork activation block.
    pub fn is_hardfork_block(&self, height: BlockHeight) -> bool {
        self.hard_forks.is_fork_block(height)
    }

    /// All upcoming upgrades (hard + soft) from `height` onward.
    pub fn upcoming_upgrades(&self, height: BlockHeight) -> Vec<UpgradeInfo> {
        let mut upgrades = Vec::new();

        if let Some(hf) = self.hard_forks.next_fork(height) {
            let remaining = hf.activation_height.as_u64()
                .saturating_sub(height.as_u64());
            upgrades.push(UpgradeInfo {
                fork_type:         ForkType::Hard,
                name:              hf.name.clone(),
                activation_height: hf.activation_height,
                blocks_remaining:  remaining,
                description:       hf.description.clone(),
            });
        }

        if let Some(sf) = self.soft_forks.next_fork(height) {
            let remaining = sf.activation_height.as_u64()
                .saturating_sub(height.as_u64());
            upgrades.push(UpgradeInfo {
                fork_type:         ForkType::Soft,
                name:              sf.name.clone(),
                activation_height: sf.activation_height,
                blocks_remaining:  remaining,
                description:       sf.description.clone(),
            });
        }

        upgrades
    }

    /// Warn if a fork activates within `warn_blocks` of the current height.
    pub fn check_upcoming_warning(
        &self,
        height:      BlockHeight,
        warn_blocks: u64,
    ) -> Vec<String> {
        self.upcoming_upgrades(height)
            .into_iter()
            .filter(|u| u.blocks_remaining <= warn_blocks)
            .map(|u| format!(
                "[UPGRADE] {:?} fork '{}' activates in {} blocks (height {})",
                u.fork_type, u.name, u.blocks_remaining,
                u.activation_height.as_u64()
            ))
            .collect()
    }
}

impl Default for UpgradeManager {
    fn default() -> Self { Self::new() }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use primitives::BlockHeight;
    use primitives::constants::{MAX_BLOCK_SIZE, MAX_TX_PER_BLOCK};

    // ── HardForkSchedule ──────────────────────────────────────────────────────

    #[test]
    fn test_genesis_fork_active_at_zero() {
        let sched = HardForkSchedule::mainnet();
        assert!(sched.is_active(1, BlockHeight::new(0)));
        assert!(sched.is_active(1, BlockHeight::new(1_000_000)));
    }

    #[test]
    fn test_unknown_fork_id_not_active() {
        let sched = HardForkSchedule::mainnet();
        assert!(!sched.is_active(999, BlockHeight::new(0)));
    }

    #[test]
    fn test_active_at_genesis() {
        let sched  = HardForkSchedule::mainnet();
        let active = sched.active_at(BlockHeight::new(0));
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].name, "Genesis");
    }

    #[test]
    fn test_no_next_fork_when_only_genesis() {
        let sched = HardForkSchedule::mainnet();
        // No forks scheduled after genesis in the current mainnet schedule.
        let next = sched.next_fork(BlockHeight::new(0));
        assert!(next.is_none());
    }

    #[test]
    fn test_is_fork_block_genesis_is_not() {
        // Genesis (id=1) is excluded from fork block detection.
        let sched = HardForkSchedule::mainnet();
        assert!(!sched.is_fork_block(BlockHeight::new(0)));
    }

    #[test]
    fn test_custom_hard_fork_activation() {
        let sched = HardForkSchedule {
            forks: vec![
                HardFork { id: 1, name: "Genesis".into(), activation_height: BlockHeight::new(0), description: "".into() },
                HardFork { id: 2, name: "Phoenix".into(), activation_height: BlockHeight::new(1000), description: "".into() },
            ],
        };

        assert!(!sched.is_active(2, BlockHeight::new(999)));
        assert!(sched.is_active(2, BlockHeight::new(1000)));
        assert!(sched.is_active(2, BlockHeight::new(9999)));
        assert!(sched.is_fork_block(BlockHeight::new(1000)));
        assert!(!sched.is_fork_block(BlockHeight::new(999)));
    }

    #[test]
    fn test_next_fork_returns_closest() {
        let sched = HardForkSchedule {
            forks: vec![
                HardFork { id: 1, name: "Genesis".into(), activation_height: BlockHeight::new(0), description: "".into() },
                HardFork { id: 2, name: "Fork2".into(), activation_height: BlockHeight::new(500), description: "".into() },
                HardFork { id: 3, name: "Fork3".into(), activation_height: BlockHeight::new(1000), description: "".into() },
            ],
        };
        let next = sched.next_fork(BlockHeight::new(400)).unwrap();
        assert_eq!(next.id, 2);
    }

    #[test]
    fn test_blocks_until_next() {
        let sched = HardForkSchedule {
            forks: vec![
                HardFork { id: 1, name: "Genesis".into(), activation_height: BlockHeight::new(0), description: "".into() },
                HardFork { id: 2, name: "Next".into(), activation_height: BlockHeight::new(100), description: "".into() },
            ],
        };
        assert_eq!(sched.blocks_until_next(BlockHeight::new(60)), Some(40));
        assert_eq!(sched.blocks_until_next(BlockHeight::new(100)), None); // already past
    }

    // ── SoftForkSchedule ──────────────────────────────────────────────────────

    #[test]
    fn test_soft_fork_empty_mainnet() {
        let sched = SoftForkSchedule::mainnet();
        assert!(sched.active_at(BlockHeight::new(0)).is_empty());
        assert!(sched.next_fork(BlockHeight::new(0)).is_none());
    }

    #[test]
    fn test_soft_fork_custom_activation() {
        let sched = SoftForkSchedule {
            forks: vec![
                SoftFork { id: 1, name: "SegWit".into(), activation_height: BlockHeight::new(500), description: "".into() },
            ],
        };
        assert!(!sched.is_active(1, BlockHeight::new(499)));
        assert!(sched.is_active(1, BlockHeight::new(500)));
    }

    // ── UpgradeManager ────────────────────────────────────────────────────────

    #[test]
    fn test_block_rules_genesis() {
        let mgr   = UpgradeManager::new();
        let rules = mgr.block_rules_at(BlockHeight::new(0));
        assert_eq!(rules.max_block_size,   MAX_BLOCK_SIZE);
        assert_eq!(rules.max_tx_per_block, MAX_TX_PER_BLOCK);
    }

    #[test]
    fn test_no_upcoming_upgrades_on_mainnet() {
        let mgr      = UpgradeManager::new();
        let upcoming = mgr.upcoming_upgrades(BlockHeight::new(0));
        // No upgrades scheduled yet on mainnet (only Genesis hard fork).
        assert!(upcoming.is_empty());
    }

    #[test]
    fn test_no_hardfork_block_on_mainnet() {
        let mgr = UpgradeManager::new();
        assert!(!mgr.is_hardfork_block(BlockHeight::new(0)));
        assert!(!mgr.is_hardfork_block(BlockHeight::new(1_000_000)));
    }

    #[test]
    fn test_upcoming_warning_with_custom_fork() {
        // Build a manager with a near-future hard fork.
        let mgr = UpgradeManager {
            hard_forks: HardForkSchedule {
                forks: vec![
                    HardFork { id: 1, name: "Genesis".into(), activation_height: BlockHeight::new(0), description: "".into() },
                    HardFork { id: 2, name: "Phoenix".into(), activation_height: BlockHeight::new(100), description: "Increases block size".into() },
                ],
            },
            soft_forks: SoftForkSchedule::mainnet(),
        };

        let warnings = mgr.check_upcoming_warning(BlockHeight::new(90), 50);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Phoenix"));
        assert!(warnings[0].contains("10 blocks"));
    }

    #[test]
    fn test_no_warning_far_away_fork() {
        let mgr = UpgradeManager {
            hard_forks: HardForkSchedule {
                forks: vec![
                    HardFork { id: 1, name: "Genesis".into(), activation_height: BlockHeight::new(0), description: "".into() },
                    HardFork { id: 2, name: "Future".into(), activation_height: BlockHeight::new(10_000), description: "".into() },
                ],
            },
            soft_forks: SoftForkSchedule::mainnet(),
        };
        // Warn only within 100 blocks — fork is 9,900 blocks away.
        let warnings = mgr.check_upcoming_warning(BlockHeight::new(100), 100);
        assert!(warnings.is_empty());
    }
}
