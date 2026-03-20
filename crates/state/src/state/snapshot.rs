// state/snapshot.rs — World state snapshot (checkpoint).
//
// A snapshot captures the complete world state at a specific block height.
// It is used for:
//   1. Rollback during chain reorganisation (restore to pre-fork state).
//   2. Fast sync (new nodes download a snapshot instead of replaying all blocks).
//   3. Periodic checkpointing to bound the depth of possible rollbacks.
//
// Snapshots are stored by the storage layer. The state layer only defines
// the snapshot data structure and the from_state / restore methods.

use std::collections::{BTreeMap, HashMap};
use serde::{Deserialize, Serialize};

use crypto::HashDigest;
use primitives::{Amount, BlockHeight, Timestamp};

use crate::account::account::Account;

/// A complete snapshot of the world state at a given block height.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// The block height this snapshot was taken after.
    pub block_height: BlockHeight,

    /// The state root (Merkle root) at this height.
    pub state_root: HashDigest,

    /// All account states at this height.
    /// Keyed by checksum hex for serde compatibility.
    pub accounts: BTreeMap<String, Account>,

    /// Total supply at this height.
    pub total_supply: Amount,

    /// Wall-clock time when the snapshot was created.
    pub created_at: Timestamp,
}

impl StateSnapshot {
    /// Capture the current world state as a snapshot.
    ///
    /// Called by the consensus layer after each block is committed.
    pub fn capture(
        block_height: BlockHeight,
        state_root:   HashDigest,
        accounts:     BTreeMap<String, Account>,
        total_supply: Amount,
    ) -> Self {
        StateSnapshot {
            block_height,
            state_root,
            accounts,
            total_supply,
            created_at: Timestamp::now(),
        }
    }

    /// Number of accounts in this snapshot.
    pub fn account_count(&self) -> usize {
        self.accounts.len()
    }
}

impl std::fmt::Display for StateSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Snapshot[{}] accounts={} supply={} root={}",
            self.block_height,
            self.accounts.len(),
            self.total_supply,
            hex::encode(&self.state_root.as_bytes()[..8]),
        )
    }
}
