// lib.rs — Public API for the wallet crate.
//
//   use wallet::{Wallet, Mnemonic, HdWallet, AddressBook, TxSigner};
//   use wallet::{Keystore, WalletError};

mod error;
mod mnemonic;
mod hd_wallet;
mod address_book;
mod signing;

mod wallet {
    pub(crate) mod keystore;
    pub(crate) mod wallet;
}

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use error::WalletError;
pub use mnemonic::Mnemonic;
pub use hd_wallet::HdWallet;
pub use address_book::{AddressBook, AddressEntry};
pub use signing::TxSigner;
pub use wallet::wallet::Wallet;
pub use wallet::keystore::Keystore;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crypto::{Address, KeyPair};
    use primitives::{Amount, Nonce};
    use primitives::constants::MIN_TX_FEE_MICRO;

    fn make_address() -> Address {
        Address::from_public_key(KeyPair::generate().unwrap().public_key())
    }

    // ── Mnemonic ──────────────────────────────────────────────────────────────

    #[test]
    fn test_mnemonic_generate_word_count() {
        let m = Mnemonic::generate();
        assert_eq!(m.word_count(), 12);
    }

    #[test]
    fn test_mnemonic_from_phrase_valid() {
        let m1 = Mnemonic::generate();
        let m2 = Mnemonic::from_phrase(m1.phrase()).unwrap();
        assert_eq!(m1.phrase(), m2.phrase());
    }

    #[test]
    fn test_mnemonic_from_phrase_wrong_word_count() {
        let result = Mnemonic::from_phrase("abandon ability");
        assert!(matches!(result, Err(WalletError::InvalidMnemonic(_))));
    }

    #[test]
    fn test_mnemonic_from_phrase_unknown_word() {
        // Build a phrase with 12 words but one unknown.
        let words = vec!["abandon"; 11];
        let mut phrase = words.join(" ");
        phrase.push_str(" xxxxxxxx");
        let result = Mnemonic::from_phrase(&phrase);
        assert!(matches!(result, Err(WalletError::InvalidMnemonic(_))));
    }

    #[test]
    fn test_mnemonic_to_seed_deterministic() {
        let m1 = Mnemonic::generate();
        let m2 = Mnemonic::from_phrase(m1.phrase()).unwrap();
        assert_eq!(*m1.to_seed(), *m2.to_seed());
    }

    #[test]
    fn test_mnemonic_different_phrases_different_seeds() {
        let m1 = Mnemonic::generate();
        let m2 = Mnemonic::generate();
        assert_ne!(*m1.to_seed(), *m2.to_seed());
    }

    // ── HdWallet ──────────────────────────────────────────────────────────────

    #[cfg(feature = "test-utils")]
    #[test]
    fn test_hd_wallet_deterministic() {
        let m  = Mnemonic::generate();
        let hd = HdWallet::from_mnemonic(&m);

        let kp1 = hd.derive_keypair(0).unwrap();
        let kp2 = hd.derive_keypair(0).unwrap();
        assert_eq!(kp1.public_key(), kp2.public_key());
    }

    #[cfg(feature = "test-utils")]
    #[test]
    fn test_hd_wallet_different_indices_differ() {
        let m  = Mnemonic::generate();
        let hd = HdWallet::from_mnemonic(&m);

        let addr0 = hd.derive_address(0).unwrap();
        let addr1 = hd.derive_address(1).unwrap();
        assert_ne!(addr0, addr1);
    }

    // ── Wallet ────────────────────────────────────────────────────────────────

    #[test]
    fn test_wallet_new_has_one_account() {
        let w = Wallet::new().unwrap();
        assert_eq!(w.account_count(), 1);
    }

    #[test]
    fn test_wallet_active_address_is_valid() {
        let w    = Wallet::new().unwrap();
        let addr = w.active_address();
        // Should be a valid address (re-derive and compare).
        let hex  = addr.to_checksum_hex();
        assert_eq!(hex.len(), 42);
        assert!(hex.starts_with("0x"));
    }

    #[test]
    fn test_wallet_add_account() {
        let mut w    = Wallet::new().unwrap();
        let new_addr = w.add_account().unwrap();
        assert_eq!(w.account_count(), 2);
        assert_ne!(w.active_address(), &new_addr);
    }

    #[test]
    fn test_wallet_set_active() {
        let mut w = Wallet::new().unwrap();
        w.add_account().unwrap();
        w.set_active(1).unwrap();
        assert_eq!(w.active_account().index(), 1);
    }

    #[test]
    fn test_wallet_set_active_out_of_bounds() {
        let mut w = Wallet::new().unwrap();
        let result = w.set_active(99);
        assert!(matches!(result, Err(WalletError::AccountNotFound(99))));
    }

    #[test]
    fn test_wallet_from_mnemonic_restore() {
        let w1     = Wallet::new().unwrap();
        let phrase = w1.backup_phrase().to_owned();

        let w2 = Wallet::from_mnemonic(&phrase).unwrap();
        // Both wallets should have the same first address.
        assert_eq!(w1.active_address(), w2.active_address());
    }

    // ── Keystore ──────────────────────────────────────────────────────────────

    #[test]
    fn test_keystore_encrypt_decrypt_roundtrip() {
        let kp  = KeyPair::generate().unwrap();
        let ks  = Keystore::encrypt(&kp, "my_password").unwrap();
        let dec = ks.decrypt_bytes("my_password").unwrap();
        assert!(!dec.is_empty());
    }

    #[test]
    fn test_keystore_wrong_password_fails() {
        let kp  = KeyPair::generate().unwrap();
        let ks  = Keystore::encrypt(&kp, "correct_password").unwrap();
        let res = ks.decrypt_bytes("wrong_password");
        assert!(matches!(res, Err(WalletError::InvalidPassword)));
    }

    #[test]
    fn test_keystore_json_roundtrip() {
        let kp   = KeyPair::generate().unwrap();
        let ks   = Keystore::encrypt(&kp, "pass").unwrap();
        let json = ks.to_json().unwrap();
        let ks2  = Keystore::from_json(&json).unwrap();
        assert_eq!(ks.address, ks2.address);
    }

    #[test]
    fn test_keystore_corrupted_fails() {
        let kp  = KeyPair::generate().unwrap();
        let mut ks = Keystore::encrypt(&kp, "pass").unwrap();
        // Corrupt the MAC.
        ks.mac = "00".repeat(32);
        let res = ks.decrypt_bytes("pass");
        assert!(matches!(res, Err(WalletError::InvalidPassword)));
    }

    // ── AddressBook ───────────────────────────────────────────────────────────

    #[test]
    fn test_address_book_add_and_find() {
        let mut book = AddressBook::new();
        let addr     = make_address();
        book.add("Alice".into(), addr.clone(), Some("friend".into())).unwrap();

        let entry = book.find_by_label("Alice").unwrap();
        assert_eq!(entry.address, addr);
        assert_eq!(entry.note.as_deref(), Some("friend"));
    }

    #[test]
    fn test_address_book_duplicate_label_rejected() {
        let mut book = AddressBook::new();
        let addr     = make_address();
        book.add("Bob".into(), addr.clone(), None).unwrap();
        let result = book.add("Bob".into(), make_address(), None);
        assert!(matches!(result, Err(WalletError::DuplicateLabel(_))));
    }

    #[test]
    fn test_address_book_remove() {
        let mut book = AddressBook::new();
        book.add("Carol".into(), make_address(), None).unwrap();
        assert_eq!(book.len(), 1);
        assert!(book.remove("Carol"));
        assert!(book.is_empty());
    }

    #[test]
    fn test_address_book_find_by_address() {
        let mut book = AddressBook::new();
        let addr     = make_address();
        book.add("Dave".into(), addr.clone(), None).unwrap();
        assert!(book.find_by_address(&addr).is_some());
        assert!(book.find_by_address(&make_address()).is_none());
    }

    // ── TxSigner ──────────────────────────────────────────────────────────────

    #[test]
    fn test_build_and_sign_transaction() {
        let kp  = KeyPair::generate().unwrap();
        let to  = make_address();
        let tx  = TxSigner::build_and_sign(
            &kp,
            to,
            Amount::from_tokens(1).unwrap(),
            Amount::from_micro(MIN_TX_FEE_MICRO).unwrap(),
            Nonce::new(1),
            None,
        ).unwrap();
        assert_eq!(tx.amount(), Amount::from_tokens(1).unwrap());
        assert_eq!(tx.nonce(), Nonce::new(1));
    }

    #[test]
    fn test_estimate_fee_base() {
        let fee = TxSigner::estimate_fee(0);
        assert_eq!(fee.as_micro(), MIN_TX_FEE_MICRO);
    }

    #[test]
    fn test_estimate_fee_with_data() {
        let fee = TxSigner::estimate_fee(100);
        assert!(fee.as_micro() > MIN_TX_FEE_MICRO);
    }
}
