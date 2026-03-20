// signing.rs — Transaction construction and signing helpers.

use crypto::{Address, KeyPair};
use primitives::{Amount, Nonce};
use transaction::{Transaction, TransactionBuilder};

use crate::error::WalletError;

pub struct TxSigner;

impl TxSigner {
    /// Build and sign a transaction.
    pub fn build_and_sign(
        keypair: KeyPair,
        to:      Address,
        amount:  Amount,
        fee:     Amount,
        nonce:   Nonce,
        data:    Option<Vec<u8>>,
    ) -> Result<Transaction, WalletError> {
        let mut builder = TransactionBuilder::new()
            .from_keypair(keypair)
            .to(to)
            .amount(amount)
            .fee(fee)
            .nonce(nonce);

        if let Some(d) = data {
            builder = builder.data(d);
        }

        builder.build().map_err(|_| WalletError::SigningFailed)
    }

    /// Estimate fee based on approximate transaction size.
    ///
    /// fee = MIN_TX_FEE + (size_bytes × FEE_PER_BYTE)
    pub fn estimate_fee(data_size: usize) -> Amount {
        use primitives::constants::MIN_TX_FEE_MICRO;
        // Base fee + 1 micro-token per extra byte of data.
        let extra = data_size as u64;
        Amount::from_micro(MIN_TX_FEE_MICRO + extra)
            .unwrap_or_else(|_| Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
    }
}
