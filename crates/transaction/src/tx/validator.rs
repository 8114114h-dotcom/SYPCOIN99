// tx/validator.rs — Transaction validation.
//
// Two validation levels:
//
//   validate_structure()
//   • Pure — no external state needed.
//   • Checks: version, chain_id, size, data size, non-zero amount,
//     fee minimum, no self-transfer, timestamp drift, valid signature.
//   • Called on mempool admission AND block validation.
//
//   validate_against_state()
//   • Requires the sender's balance and nonce from the state layer.
//   • Checks: sufficient balance (amount + fee), correct nonce sequence.
//   • Called during block execution (state transition).
//
// Separation rationale:
//   Structural validation is stateless and cheap — run it first to reject
//   malformed transactions before any state lookup. State validation is more
//   expensive (DB read) and only makes sense for structurally valid transactions.

use crypto::{Address, NoncePayload, verify};
use primitives::{Amount, Nonce, Timestamp};
use primitives::constants::{CHAIN_ID, MIN_TX_FEE_MICRO};

use crate::error::TransactionError;
use crate::tx::constants::{MAX_TX_DATA_SIZE, MAX_TX_FUTURE_TIME_MS, MAX_TX_SIZE_BYTES, TX_VERSION};
use crate::tx::transaction::Transaction;

pub struct TransactionValidator;

impl TransactionValidator {
    // ── Structural validation (stateless) ────────────────────────────────────

    /// Validate a transaction's structure and signature.
    ///
    /// Does NOT check balance or nonce — those require state.
    pub fn validate_structure(tx: &Transaction) -> Result<(), TransactionError> {
        // 1. Protocol version must be supported.
        if tx.version() != TX_VERSION {
            return Err(TransactionError::InvalidVersion(tx.version()));
        }

        // 2. Chain ID must match this network.
        if tx.chain_id() != CHAIN_ID {
            return Err(TransactionError::InvalidChainId {
                expected: CHAIN_ID,
                got:      tx.chain_id(),
            });
        }

        // 3. Amount must be non-zero.
        if tx.amount().is_zero() {
            return Err(TransactionError::AmountIsZero);
        }

        // 4. Fee must meet the minimum.
        let min_fee = Amount::from_micro(MIN_TX_FEE_MICRO)
            .map_err(|_| TransactionError::AmountOverflow)?;
        if tx.fee() < min_fee {
            return Err(TransactionError::InsufficientFee {
                minimum:  min_fee,
                provided: tx.fee(),
            });
        }

        // 5. No self-transfers.
        if tx.from() == tx.to() {
            return Err(TransactionError::SelfTransfer);
        }

        // 6. Data payload size.
        if let Some(data) = tx.data() {
            if data.len() > MAX_TX_DATA_SIZE {
                return Err(TransactionError::DataTooLarge {
                    max: MAX_TX_DATA_SIZE,
                    got: data.len(),
                });
            }
        }

        // 7. Serialized transaction size.
        let size = tx.size_bytes();
        if size > MAX_TX_SIZE_BYTES {
            return Err(TransactionError::TransactionTooLarge {
                max: MAX_TX_SIZE_BYTES,
                got: size,
            });
        }

        // 8. Timestamp must not be too far in the future.
        let now = Timestamp::now();
        tx.timestamp()
            .validate_not_future(&now, MAX_TX_FUTURE_TIME_MS)
            .map_err(|_| TransactionError::TimestampInFuture)?;

        // 9. from address must match the public key.
        //    Prevents a transaction claiming to be from address A
        //    while including public key B.
        let derived = Address::from_public_key(tx.public_key());
        if &derived != tx.from() {
            return Err(TransactionError::InvalidSignature);
        }

        // 10. Cryptographic signature verification.
        //     We rebuild the exact NoncePayload that was signed in the builder.
        let signing_bytes  = tx.to_bytes_for_signing();
        let nonce_payload  = NoncePayload::new(tx.nonce().as_u64(), signing_bytes);
        verify(tx.public_key(), &nonce_payload, tx.signature())
            .map_err(|_| TransactionError::InvalidSignature)?;

        Ok(())
    }

    // ── State validation (requires account state) ─────────────────────────────

    /// Validate a transaction against the sender's current account state.
    ///
    /// # Arguments
    /// - `sender_balance` — current confirmed balance of the sender.
    /// - `sender_nonce`   — current confirmed nonce of the sender.
    /// - `current_time`   — current wall-clock time (for expiry check).
    pub fn validate_against_state(
        tx:             &Transaction,
        sender_balance: Amount,
        sender_nonce:   Nonce,
        current_time:   Timestamp,
    ) -> Result<(), TransactionError> {
        // 1. Nonce must exactly follow the account's current nonce.
        let expected_nonce = sender_nonce
            .next()
            .map_err(|_| TransactionError::InvalidNonce {
                expected: sender_nonce,
                got:      tx.nonce(),
            })?;

        if tx.nonce() != expected_nonce {
            return Err(TransactionError::InvalidNonce {
                expected: expected_nonce,
                got:      tx.nonce(),
            });
        }

        // 2. Sender must have enough balance to cover amount + fee.
        let total = tx
            .total_deducted()
            .ok_or(TransactionError::AmountOverflow)?;

        if sender_balance < total {
            return Err(TransactionError::InsufficientBalance {
                available: sender_balance,
                required:  total,
            });
        }

        // 3. Transaction must not have expired (TTL check).
        //    If the tx timestamp is older than TX_TTL_MS ago, reject it.
        use crate::tx::constants::TX_TTL_MS;
        if let Some(age_ms) = current_time.millis_since(&tx.timestamp()) {
            if age_ms > TX_TTL_MS {
                return Err(TransactionError::TransactionExpired);
            }
        }

        Ok(())
    }
}
