// tx/builder.rs — TransactionBuilder.
//
// Security decisions:
//
//   SINGLE CREATION PATH
//   • TransactionBuilder::build() is the ONLY way to produce a Transaction.
//     It enforces all structural invariants before returning:
//       1. All required fields are set.
//       2. amount > 0, fee >= MIN_TX_FEE.
//       3. from != to (no self-transfer).
//       4. data.len() <= MAX_TX_DATA_SIZE.
//       5. Signature is computed and embedded.
//       6. tx_id is computed from the final canonical bytes.
//
//   CHAIN_ID BINDING
//   • chain_id is set from primitives::constants::CHAIN_ID automatically.
//     The caller cannot override it. This ensures every transaction built
//     by this node is bound to the correct chain.
//
//   FROM ADDRESS DERIVED — NOT SUPPLIED
//   • The `from` address is always derived from the keypair's public key:
//     `Address::from_public_key(&keypair.public_key())`.
//     A caller cannot supply a different `from` address — doing so would
//     allow them to sign a transaction claiming to be from an address they
//     don't control.
//
//   SIGNING HAPPENS IN BUILD
//   • The NoncePayload fed to sign() uses the nonce from the Nonce field
//     and the canonical to_bytes() as the payload. This means the signature
//     covers every field of the transaction including chain_id and timestamp.

use crypto::{Address, KeyPair, NoncePayload, sign};
use primitives::{Amount, Nonce, Timestamp};
use primitives::constants::{CHAIN_ID, MIN_TX_FEE_MICRO};

use crate::error::TransactionError;
use crate::tx::constants::{MAX_TX_DATA_SIZE, MAX_TX_SIZE_BYTES, MAX_TX_FUTURE_TIME_MS, TX_VERSION};
use crate::tx::transaction::Transaction;

/// Builder for constructing a signed [`Transaction`].
///
/// All fields except `data` are required. `build()` will return
/// `Err(MissingField)` if any required field is absent.
///
/// # Example
/// ```ignore
/// let tx = TransactionBuilder::new()
///     .from_keypair(&keypair)
///     .to(recipient_address)
///     .amount(Amount::from_tokens(10).unwrap())
///     .fee(Amount::from_micro(1_000).unwrap())
///     .nonce(Nonce::new(0))
///     .build()?;
/// ```
#[derive(Default)]
pub struct TransactionBuilder {
    keypair:   Option<KeyPair>,
    to:        Option<Address>,
    amount:    Option<Amount>,
    fee:       Option<Amount>,
    nonce:     Option<Nonce>,
    timestamp: Option<Timestamp>,
    data:      Option<Vec<u8>>,
}

impl TransactionBuilder {
    /// Create a new empty builder.
    pub fn new() -> Self {
        TransactionBuilder::default()
    }

    /// Set the sender keypair.
    ///
    /// Derives `from` address and `public_key` automatically.
    /// This is the only way to set the sender — no raw address accepted.
    pub fn from_keypair(mut self, keypair: KeyPair) -> Self {
        self.keypair = Some(keypair);
        self
    }

    /// Set the recipient address.
    pub fn to(mut self, address: Address) -> Self {
        self.to = Some(address);
        self
    }

    /// Set the transfer amount.
    pub fn amount(mut self, amount: Amount) -> Self {
        self.amount = Some(amount);
        self
    }

    /// Set the transaction fee.
    pub fn fee(mut self, fee: Amount) -> Self {
        self.fee = Some(fee);
        self
    }

    /// Set the sender's current account nonce.
    pub fn nonce(mut self, nonce: Nonce) -> Self {
        self.nonce = Some(nonce);
        self
    }

    /// Set the transaction timestamp.
    ///
    /// If not set, defaults to `Timestamp::now()` at `build()` time.
    pub fn timestamp(mut self, timestamp: Timestamp) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Set optional arbitrary data payload (max MAX_TX_DATA_SIZE bytes).
    pub fn data(mut self, data: Vec<u8>) -> Self {
        self.data = Some(data);
        self
    }

    /// Validate all fields, sign the transaction, and return a [`Transaction`].
    ///
    /// # Errors
    /// - `MissingField` if any required field is absent.
    /// - `AmountIsZero` if amount == 0.
    /// - `InsufficientFee` if fee < MIN_TX_FEE_MICRO.
    /// - `SelfTransfer` if from == to.
    /// - `DataTooLarge` if data.len() > MAX_TX_DATA_SIZE.
    /// - `TransactionTooLarge` if serialized size > MAX_TX_SIZE_BYTES.
    /// - `TimestampInFuture` if timestamp is too far ahead.
    /// - `AmountOverflow` if amount + fee overflows.
    /// - `SigningFailed` if the OS entropy source is unavailable.
    pub fn build(self) -> Result<Transaction, TransactionError> {
        // ── 1. Unwrap required fields ─────────────────────────────────────────
        let keypair = self.keypair
            .ok_or_else(|| TransactionError::MissingField("keypair".into()))?;
        let to = self.to
            .ok_or_else(|| TransactionError::MissingField("to".into()))?;
        let amount = self.amount
            .ok_or_else(|| TransactionError::MissingField("amount".into()))?;
        let fee = self.fee
            .ok_or_else(|| TransactionError::MissingField("fee".into()))?;
        let nonce = self.nonce
            .ok_or_else(|| TransactionError::MissingField("nonce".into()))?;

        // Timestamp defaults to now if not explicitly set.
        let timestamp = self.timestamp.unwrap_or_else(Timestamp::now);

        // ── 2. Derive from address from keypair ───────────────────────────────
        let public_key = keypair.public_key().clone();
        let from       = Address::from_public_key(&public_key);

        // ── 3. Structural validations ─────────────────────────────────────────

        // No zero-value transfers.
        if amount.is_zero() {
            return Err(TransactionError::AmountIsZero);
        }

        // Fee must meet the network minimum.
        let min_fee = Amount::from_micro(MIN_TX_FEE_MICRO)
            .map_err(|_| TransactionError::AmountOverflow)?;
        if fee < min_fee {
            return Err(TransactionError::InsufficientFee {
                minimum:  min_fee,
                provided: fee,
            });
        }

        // No self-transfers.
        if from == to {
            return Err(TransactionError::SelfTransfer);
        }

        // Ensure amount + fee does not overflow.
        amount.checked_add(fee).ok_or(TransactionError::AmountOverflow)?;

        // Data payload size check.
        if let Some(ref d) = self.data {
            if d.len() > MAX_TX_DATA_SIZE {
                return Err(TransactionError::DataTooLarge {
                    max: MAX_TX_DATA_SIZE,
                    got: d.len(),
                });
            }
        }

        // Timestamp must not be too far in the future.
        let now = Timestamp::now();
        timestamp
            .validate_not_future(&now, MAX_TX_FUTURE_TIME_MS)
            .map_err(|_| TransactionError::TimestampInFuture)?;

        // ── 4. Build unsigned transaction (no sig / tx_id yet) ────────────────
        // We need to_bytes() to build the signing pre-image, but to_bytes()
        // reads self.signature. We use a placeholder signature temporarily —
        // the signature field is NOT part of the signed pre-image (the pre-image
        // is the canonical fields WITHOUT the signature). This is safe because
        // we compute to_bytes_for_signing() which excludes the signature field.
        let mut tx = Transaction {
            version:    TX_VERSION,
            chain_id:   CHAIN_ID,
            from:       from.clone(),
            to:         to.clone(),
            public_key: public_key.clone(),
            amount,
            fee,
            nonce,
            // Placeholder — will be replaced after signing.
            signature:  crypto::Signature::from_bytes([0u8; 64])
                .map_err(|_| TransactionError::SigningFailed)?,
            timestamp,
            data:       self.data.clone(),
            tx_id:      crypto::sha256(&[]), // placeholder
        };

        // ── 5. Compute the bytes to sign (excludes signature and tx_id) ───────
        let signing_bytes = tx.to_bytes_for_signing();

        // ── 6. Sign via the crypto crate ──────────────────────────────────────
        // NoncePayload binds the account nonce to the signing pre-image.
        // The payload is the canonical transaction bytes (minus signature).
        // chain_id is already in those bytes → cross-chain replay prevented.
        let nonce_payload = NoncePayload::new(nonce.as_u64(), signing_bytes);
        let signature = sign(&keypair, &nonce_payload)
            .map_err(|_| TransactionError::SigningFailed)?;

        tx.signature = signature;

        // ── 7. Compute tx_id from the complete canonical bytes ─────────────────
        // to_bytes() now includes the real signature.
        let canonical = tx.to_bytes();

        // Final size check after full serialization.
        if canonical.len() > MAX_TX_SIZE_BYTES {
            return Err(TransactionError::TransactionTooLarge {
                max: MAX_TX_SIZE_BYTES,
                got: canonical.len(),
            });
        }

        tx.tx_id = Transaction::compute_tx_id(&canonical);

        Ok(tx)
    }
}
