// journal.rs — Change journal for atomic rollback.
//
// Security decisions:
//
//   RECORD BEFORE MUTATE
//   • Every mutation in WorldState records the PREVIOUS value in the journal
//     BEFORE applying the change. This ensures rollback always has the
//     correct before-state regardless of the order of mutations.
//
//   ATOMIC TRANSACTIONS
//   • A transaction either fully succeeds (journal.clear() called) or
//     fully rolls back (journal.rollback() called). There is no partial state.
//
//   JOURNAL ORDERING
//   • rollback() replays entries in REVERSE order. This matters when the
//     same account is modified multiple times in one transaction (e.g. sender
//     pays fee then amount — rollback must undo amount first, then fee).
//
//   USAGE PATTERN:
//     1. journal.record(old_state)   ← before any mutation
//     2. apply mutation to state
//     3a. success → journal.clear()
//     3b. failure → journal.rollback(&mut state)

use serde::{Deserialize, Serialize};

use crypto::Address;
use primitives::{Amount, Nonce};

use crate::account::store::AccountStore;

/// A single recorded state change that can be undone.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum JournalEntry {
    /// An account's balance changed.
    BalanceChanged {
        address: String,  // checksum hex
        old:     Amount,
        new:     Amount,
    },
    /// An account's nonce changed.
    NonceChanged {
        address: String,
        old:     Nonce,
        new:     Nonce,
    },
    /// An account was created for the first time (balance=0, nonce=0).
    AccountCreated {
        address: String,
    },
}

/// An ordered log of state changes for a single transaction or block.
///
/// Used to roll back changes if a transaction or block fails mid-execution.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Journal {
    entries: Vec<JournalEntry>,
}

impl Journal {
    pub fn new() -> Self {
        Journal::default()
    }

    /// Record a state change. Must be called BEFORE the mutation is applied.
    pub fn record(&mut self, entry: JournalEntry) {
        self.entries.push(entry);
    }

    /// Record a balance change for an address.
    pub fn record_balance_change(&mut self, address: &Address, old: Amount, new: Amount) {
        self.entries.push(JournalEntry::BalanceChanged {
            address: address.to_checksum_hex(),
            old,
            new,
        });
    }

    /// Record a nonce change for an address.
    pub fn record_nonce_change(&mut self, address: &Address, old: Nonce, new: Nonce) {
        self.entries.push(JournalEntry::NonceChanged {
            address: address.to_checksum_hex(),
            old,
            new,
        });
    }

    /// Record the creation of a new account.
    pub fn record_account_created(&mut self, address: &Address) {
        self.entries.push(JournalEntry::AccountCreated {
            address: address.to_checksum_hex(),
        });
    }

    /// Undo all recorded changes in reverse order.
    ///
    /// After calling this, the AccountStore is restored to the state it was
    /// in before any of the journaled mutations were applied.
    pub fn rollback(&self, store: &mut AccountStore) {
        // Reverse order: undo last change first.
        for entry in self.entries.iter().rev() {
            match entry {
                JournalEntry::BalanceChanged { address, old, .. } => {
                    // Find the account by its hex key and restore old balance.
                    // We iterate because AccountStore keys by checksum hex.
                    if let Some(acc) = store.get_by_hex_mut(address) {
                        acc.balance = *old;
                    }
                }
                JournalEntry::NonceChanged { address, old, .. } => {
                    if let Some(acc) = store.get_by_hex_mut(address) {
                        acc.nonce = *old;
                    }
                }
                JournalEntry::AccountCreated { address } => {
                    // Remove the account that was created — it didn't exist before.
                    store.remove_by_hex(address);
                }
            }
        }
    }

    /// Clear all journal entries after a successful commit.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Number of recorded entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
