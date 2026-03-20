// wallet/wallet.rs — The Wallet: manages multiple accounts.

use crypto::{Address, KeyPair};

use crate::error::WalletError;
use crate::hd_wallet::HdWallet;
use crate::mnemonic::Mnemonic;
use crate::wallet::keystore::Keystore;

/// A single account within the wallet.
pub struct WalletAccount {
    pub name:    String,
    pub address: Address,
    pub index:   u32,           // HD derivation index
    keypair:     KeyPair,       // private — never exposed directly
}

impl WalletAccount {
    pub fn address(&self) -> &Address { &self.address }
    pub fn keypair(&self)  -> &KeyPair { &self.keypair }
    pub fn name(&self)     -> &str     { &self.name }
    pub fn index(&self)    -> u32      { self.index }
}

/// A multi-account HD wallet.
///
/// All accounts are derived from a single mnemonic seed.
pub struct Wallet {
    hd:       HdWallet,
    accounts: Vec<WalletAccount>,
    active:   usize,
    mnemonic: Mnemonic,         // kept for export / backup
}

impl Wallet {
    /// Create a new wallet with a freshly generated mnemonic.
    /// Derives the first account (index 0) automatically.
    pub fn new() -> Result<Self, WalletError> {
        let mnemonic = Mnemonic::generate();
        Self::from_mnemonic_inner(mnemonic)
    }

    /// Restore a wallet from an existing mnemonic phrase.
    pub fn from_mnemonic(phrase: &str) -> Result<Self, WalletError> {
        let mnemonic = Mnemonic::from_phrase(phrase)?;
        Self::from_mnemonic_inner(mnemonic)
    }

    fn from_mnemonic_inner(mnemonic: Mnemonic) -> Result<Self, WalletError> {
        let hd = HdWallet::from_mnemonic(&mnemonic);

        // Derive first account.
        let kp      = hd.derive_keypair(0)?;
        let address = Address::from_public_key(kp.public_key());
        let account = WalletAccount {
            name:    "Account 0".into(),
            address,
            index:   0,
            keypair: kp,
        };

        Ok(Wallet {
            hd,
            accounts: vec![account],
            active:   0,
            mnemonic,
        })
    }

    // ── Account management ────────────────────────────────────────────────────

    /// Derive and add a new account at the next index.
    pub fn add_account(&mut self) -> Result<Address, WalletError> {
        let next_index = self.accounts.len() as u32;
        let kp         = self.hd.derive_keypair(next_index)?;
        let address    = Address::from_public_key(kp.public_key());

        self.accounts.push(WalletAccount {
            name:    format!("Account {}", next_index),
            address: address.clone(),
            index:   next_index,
            keypair: kp,
        });

        Ok(address)
    }

    pub fn get_account(&self, idx: usize) -> Result<&WalletAccount, WalletError> {
        self.accounts.get(idx).ok_or(WalletError::AccountNotFound(idx))
    }

    pub fn active_account(&self) -> &WalletAccount {
        &self.accounts[self.active]
    }

    pub fn active_address(&self) -> &Address {
        self.active_account().address()
    }

    pub fn set_active(&mut self, idx: usize) -> Result<(), WalletError> {
        if idx >= self.accounts.len() {
            return Err(WalletError::AccountNotFound(idx));
        }
        self.active = idx;
        Ok(())
    }

    pub fn account_count(&self) -> usize { self.accounts.len() }

    // ── Backup ────────────────────────────────────────────────────────────────

    /// The mnemonic phrase for backup. Keep this secret.
    pub fn backup_phrase(&self) -> &str {
        self.mnemonic.phrase()
    }

    /// Export the active account's keypair as an encrypted keystore.
    pub fn export_keystore(
        &self,
        account_idx: usize,
        password:    &str,
    ) -> Result<String, WalletError> {
        let account = self.get_account(account_idx)?;
        let ks      = Keystore::encrypt(account.keypair(), password)?;
        ks.to_json()
    }
}
