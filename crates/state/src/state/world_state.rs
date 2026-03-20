// state/world_state.rs — The complete world state.
//
// WorldState is the single source of truth for all account data.
// It owns:
//   • AccountStore — all accounts indexed by address
//   • Journal      — change log for rollback
//   • StateCache   — hot-account read cache
//   • total_supply — sum of all balances (increases only via block reward)
//   • state_root   — Merkle commitment to all accounts
//   • block_height — last committed block

use std::collections::{BTreeMap, HashMap};

use crypto::{Address, HashDigest};
use primitives::{Amount, BlockHeight, Nonce};
use transaction::Transaction;

use crate::account::account::Account;
use crate::account::store::AccountStore;
use crate::cache::StateCache;
use crate::error::StateError;
use crate::journal::Journal;
use crate::state::snapshot::StateSnapshot;
use crate::state::transition::StateTransition;
use crate::trie::merkle_trie::compute_state_root;

/// Default LRU cache capacity (number of accounts).
const DEFAULT_CACHE_CAPACITY: usize = 1_024;

/// The complete, authoritative world state of the blockchain.
///
/// # Thread Safety
/// WorldState is NOT thread-safe. The node layer must ensure exclusive
/// access during block execution. Concurrent reads can be served from
/// a read-only snapshot.
pub struct WorldState {
    store:        AccountStore,
    journal:      Journal,
    cache:        StateCache,
    total_supply: Amount,
    state_root:   HashDigest,
    block_height: BlockHeight,
}

impl WorldState {
    /// Create a new empty world state (genesis).
    pub fn new() -> Self {
        let empty_root = crypto::sha256(b"SYPCOIN_STATE_EMPTY_V1");
        WorldState {
            store:        AccountStore::new(),
            journal:      Journal::new(),
            cache:        StateCache::new(DEFAULT_CACHE_CAPACITY),
            total_supply: Amount::ZERO,
            state_root:   empty_root,
            block_height: BlockHeight::GENESIS,
        }
    }

    // ── Read-only queries ─────────────────────────────────────────────────────

    /// Get an account by address. Returns `None` if it doesn't exist.
    pub fn get_account(&self, addr: &Address) -> Option<&Account> {
        self.store.get(addr)
    }

    /// Get the balance of an address (0 if account doesn't exist).
    pub fn get_balance(&self, addr: &Address) -> Amount {
        self.store.get(addr).map(|a| a.balance()).unwrap_or(Amount::ZERO)
    }

    /// Get the nonce of an address (0 if account doesn't exist).
    pub fn get_nonce(&self, addr: &Address) -> Nonce {
        self.store.get(addr).map(|a| a.nonce()).unwrap_or(Nonce::ZERO)
    }

    /// Total supply of tokens in circulation.
    pub fn total_supply(&self) -> Amount {
        self.total_supply
    }

    /// Merkle root committing to all account states.
    pub fn state_root(&self) -> &HashDigest {
        &self.state_root
    }

    /// The height of the last committed block.
    pub fn block_height(&self) -> BlockHeight {
        self.block_height
    }

    /// Number of accounts in the state.
    pub fn account_count(&self) -> usize {
        self.store.len()
    }

    // ── Mutations ─────────────────────────────────────────────────────────────

    /// Apply a single transaction to the world state.
    ///
    /// On success: returns the net effect and clears the journal.
    /// On failure: rolls back all changes and returns Err.
    pub fn apply_transaction(
        &mut self,
        tx:    &Transaction,
        miner: &Address,
    ) -> Result<crate::state::transition::TxEffect, StateError> {
        // Invalidate cache entries for affected addresses.
        self.cache.invalidate(tx.from());
        self.cache.invalidate(tx.to());
        self.cache.invalidate(miner);

        StateTransition::apply_tx(
            &mut self.store,
            &mut self.journal,
            tx,
            miner,
        )
    }

    /// Apply the block reward to the miner for the given block height.
    ///
    /// Increases total_supply by the reward amount.
    pub fn apply_block_reward(
        &mut self,
        miner:  &Address,
        height: &BlockHeight,
    ) -> Result<Amount, StateError> {
        self.cache.invalidate(miner);
        StateTransition::apply_reward(
            &mut self.store,
            &mut self.journal,
            miner,
            height,
            &mut self.total_supply,
        )
    }

    /// Seed a genesis account with an initial balance.
    ///
    /// Only valid before any blocks are applied (block_height == GENESIS).
    /// Increases total_supply.
    pub fn set_genesis_balance(
        &mut self,
        addr:   Address,
        amount: Amount,
    ) -> Result<(), StateError> {
        if !self.block_height.is_genesis() {
            return Err(StateError::InvalidTransition(
                "genesis balances can only be set at height 0".into()
            ));
        }
        let new_supply = self.total_supply
            .checked_add(amount)
            .ok_or(StateError::SupplyOverflow)?;

        let acc = self.store.get_or_create(&addr);
        acc.credit(amount)?;
        self.total_supply = new_supply;
        Ok(())
    }

    /// Commit the current state: compute a new state_root and advance height.
    ///
    /// Must be called after all transactions and the block reward for a block
    /// have been applied successfully.
    pub fn commit(&mut self, new_height: BlockHeight) -> HashDigest {
        // Compute new Merkle root from all accounts.
        let accounts: Vec<&Account> = self.store.iter().collect();
        self.state_root  = compute_state_root(&accounts);
        self.block_height = new_height;
        self.journal.clear();
        self.state_root.clone()
    }

    /// Take a snapshot of the current state.
    ///
    /// Used by the consensus layer for checkpoint storage.
    pub fn snapshot(&self) -> StateSnapshot {
        // BTreeMap ensures deterministic ordering for state root.
        let accounts: BTreeMap<String, Account> = self
            .store
            .iter()
            .map(|a| (a.address().to_checksum_hex(), a.clone()))
            .collect();

        StateSnapshot::capture(
            self.block_height,
            self.state_root.clone(),
            accounts,
            self.total_supply,
        )
    }

    /// Restore from a snapshot.
    ///
    /// Replaces all current state with the snapshot contents.
    pub fn restore_from_snapshot(&mut self, snap: StateSnapshot) {
        self.store        = AccountStore::new();
        self.journal      = Journal::new();
        self.cache        = StateCache::new(DEFAULT_CACHE_CAPACITY);
        self.total_supply = snap.total_supply;
        self.state_root   = snap.state_root;
        self.block_height = snap.block_height;

        for (_, account) in snap.accounts {
            self.store.insert(account);
        }
    }

    /// Verify the supply invariant: sum of all balances == total_supply.
    ///
    /// Used in tests and during audit. O(n) in number of accounts.
    pub fn verify_supply_invariant(&self) -> bool {
        match self.store.sum_balances() {
            Some(sum) => sum == self.total_supply,
            None      => false,
        }
    }
}

impl Default for WorldState {
    fn default() -> Self {
        Self::new()
    }
}
