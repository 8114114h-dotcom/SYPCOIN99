// error.rs — Unified error type for the wallet crate.

use thiserror::Error;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum WalletError {
    #[error("invalid mnemonic phrase: {0}")]
    InvalidMnemonic(String),

    #[error("wrong password or corrupted keystore")]
    InvalidPassword,

    #[error("keystore file is corrupted")]
    KeystoreCorrupted,

    #[error("key derivation failed: {0}")]
    DerivationFailed(String),

    #[error("signing operation failed")]
    SigningFailed,

    #[error("invalid address: {0}")]
    InvalidAddress(String),

    #[error("account index {0} not found")]
    AccountNotFound(usize),

    #[error("address book is full")]
    AddressBookFull,

    #[error("label '{0}' already exists in address book")]
    DuplicateLabel(String),

    #[error("serialization error: {0}")]
    SerializationError(String),
}
