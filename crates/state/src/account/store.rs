// account/store.rs — In-memory account store.
//
// AccountStore is a thin wrapper around HashMap<Address, Account>.
// It is owned by WorldState and provides the single access point for
// reading and mutating accounts.
//
// Design:
//   • get_or_create() is the primary entry point for mutations.
//     It lazily initialises accounts with zero balance/nonce on first access,
//     matching Ethereum-style account model (no explicit "create account" tx).
//   • All returned references are &mut Account so the caller (WorldState /
//     StateTransition) can apply mutations directly. The Journal records
//     the before-state before any mutation is applied.

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

use crypto::Address;
use primitives::Amount;

use crate::account::account::Account;

/// In-memory store for all accounts in the world state.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AccountStore {
    // BTreeMap — NOT HashMap. BTreeMap guarantees deterministic iteration order,
    // which is critical for Merkle trie root computation. Using HashMap would
    // cause different nodes to compute different state roots (Consensus Failure).
    accounts: BTreeMap<String, Account>,
}

impl AccountStore {
    pub fn new() -> Self {
        AccountStore::default()
    }

    /// Look up an account by address. Returns `None` if not yet created.
    pub fn get(&self, addr: &Address) -> Option<&Account> {
        self.accounts.get(&addr.to_checksum_hex())
    }

    /// Look up an account mutably. Returns `None` if not yet created.
    pub fn get_mut(&mut self, addr: &Address) -> Option<&mut Account> {
        self.accounts.get_mut(&addr.to_checksum_hex())
    }

    /// Get or lazily create an account.
    ///
    /// New accounts start with balance=0, nonce=0.
    /// The caller must journal the AccountCreated event before mutating.
    pub fn get_or_create(&mut self, addr: &Address) -> &mut Account {
        self.accounts
            .entry(addr.to_checksum_hex())
            .or_insert_with(|| Account::new(addr.clone()))
    }

    /// Returns true if an account exists for this address.
    pub fn contains(&self, addr: &Address) -> bool {
        self.accounts.contains_key(&addr.to_checksum_hex())
    }

    /// Insert or replace an account (used during snapshot restore).
    pub fn insert(&mut self, account: Account) {
        self.accounts.insert(account.address().to_checksum_hex(), account);
    }

    /// Remove an account (used during tests / pruning).
    pub fn remove(&mut self, addr: &Address) -> Option<Account> {
        self.accounts.remove(&addr.to_checksum_hex())
    }

    /// Number of accounts in the store.
    pub fn len(&self) -> usize {
        self.accounts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }

    /// Iterate over all accounts. Used for state_root computation.
    pub fn iter(&self) -> impl Iterator<Item = &Account> {
        self.accounts.values()
    }

    /// Total balance across all accounts. Used for supply invariant checks.
    pub fn sum_balances(&self) -> Option<Amount> {
        self.accounts.values().try_fold(Amount::ZERO, |acc, a| {
            acc.checked_add(a.balance())
        })
    }
}

impl AccountStore {
    /// Internal: look up account by pre-computed checksum hex key.
    /// Used by Journal::rollback() which stores hex keys.
    pub(crate) fn get_by_hex_mut(&mut self, hex: &str) -> Option<&mut crate::account::account::Account> {
        self.accounts.get_mut(hex)
    }

    /// Internal: remove account by pre-computed checksum hex key.
    pub(crate) fn remove_by_hex(&mut self, hex: &str) {
        self.accounts.remove(hex);
    }
}
