// address_book.rs — Named address book for the wallet.

use crypto::Address;
use serde::{Deserialize, Serialize};

use crate::error::WalletError;

const MAX_ENTRIES: usize = 1_000;

/// A labelled entry in the address book.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddressEntry {
    pub label:   String,
    pub address: Address,
    pub note:    Option<String>,
}

/// A collection of named addresses.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AddressBook {
    entries: Vec<AddressEntry>,
}

impl AddressBook {
    pub fn new() -> Self { AddressBook::default() }

    /// Add a new entry. Returns `Err` if the label already exists or book is full.
    pub fn add(
        &mut self,
        label:   String,
        address: Address,
        note:    Option<String>,
    ) -> Result<(), WalletError> {
        if self.entries.len() >= MAX_ENTRIES {
            return Err(WalletError::AddressBookFull);
        }
        if self.find_by_label(&label).is_some() {
            return Err(WalletError::DuplicateLabel(label));
        }
        self.entries.push(AddressEntry { label, address, note });
        Ok(())
    }

    pub fn find_by_label(&self, label: &str) -> Option<&AddressEntry> {
        self.entries.iter().find(|e| e.label == label)
    }

    pub fn find_by_address(&self, addr: &Address) -> Option<&AddressEntry> {
        self.entries.iter().find(|e| &e.address == addr)
    }

    pub fn list(&self) -> &[AddressEntry] {
        &self.entries
    }

    /// Remove an entry by label. Returns `true` if it existed.
    pub fn remove(&mut self, label: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.label != label);
        self.entries.len() < before
    }

    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
}
