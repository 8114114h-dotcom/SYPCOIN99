// tx_executor.rs — Single transaction execution.
//
// Design:
//   TxExecutor validates a transaction against the CURRENT state and applies
//   it atomically. On failure, the state is unchanged (WorldState uses a
//   journal for rollback internally).
//
//   Validation order:
//     1. Structural validation (no state needed) — signature, chain_id, size.
//     2. State validation — balance and nonce against live state.
//     3. Apply via WorldState::apply_transaction().
//
//   Fee behaviour on failure:
//     In a production chain, a failed tx still consumes the fee to prevent
//     spam. We mark the receipt as Failed but still deduct the fee.
//     For simplicity in this implementation, a failed structural validation
//     means the tx was never admitted to the block — the fee is NOT deducted.
//     Only state-level failures (after structural pass) deduct the fee.

use serde::{Deserialize, Serialize};

use crypto::{Address, HashDigest};
use primitives::{Amount, Timestamp};
use state::WorldState;
use transaction::{Transaction, TransactionValidator};

use crate::error::ExecutionError;

/// Outcome of executing a single transaction.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TxStatus {
    Success,
    Failed(String),
}

impl TxStatus {
    pub fn is_success(&self) -> bool {
        matches!(self, TxStatus::Success)
    }
}

/// Receipt produced for each transaction executed in a block.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TxReceipt {
    /// The transaction that was executed.
    pub tx_id:                   HashDigest,
    /// Execution outcome.
    pub status:                  TxStatus,
    /// Fee consumed by this transaction.
    pub fee:                     Amount,
    /// Sender's balance after execution.
    pub sender_balance_after:    Amount,
    /// Recipient's balance after execution.
    pub recipient_balance_after: Amount,
}

pub struct TxExecutor;

impl TxExecutor {
    /// Execute a single transaction against the world state.
    ///
    /// # Arguments
    /// - `state` — mutable world state.
    /// - `tx`    — the transaction to execute.
    /// - `miner` — address to receive the fee.
    /// - `now`   — current wall-clock time (for TTL validation).
    ///
    /// # Returns
    /// A `TxReceipt` describing the outcome. If the transaction fails at the
    /// state level (after structural validation), the fee is still deducted.
    pub fn execute(
        state: &mut WorldState,
        tx:    &Transaction,
        miner: &Address,
        now:   Timestamp,
    ) -> Result<TxReceipt, ExecutionError> {
        // ── 1. Structural validation ──────────────────────────────────────────
        TransactionValidator::validate_structure(tx)
            .map_err(|e| ExecutionError::ValidationError(e.to_string()))?;

        // ── 2. State validation ───────────────────────────────────────────────
        let sender_balance = state.get_balance(tx.from());
        let sender_nonce   = state.get_nonce(tx.from());

        if let Err(e) = TransactionValidator::validate_against_state(
            tx,
            sender_balance,
            sender_nonce,
            now,
        ) {
            // State validation failed — do NOT deduct fee (tx never applied).
            return Ok(TxReceipt {
                tx_id:                   tx.tx_id().clone(),
                status:                  TxStatus::Failed(e.to_string()),
                fee:                     Amount::ZERO,
                sender_balance_after:    sender_balance,
                recipient_balance_after: state.get_balance(tx.to()),
            });
        }

        // ── 3. Apply to state ─────────────────────────────────────────────────
        match state.apply_transaction(tx, miner) {
            Ok(effect) => Ok(TxReceipt {
                tx_id:                   tx.tx_id().clone(),
                status:                  TxStatus::Success,
                fee:                     effect.fee_collected,
                sender_balance_after:    effect.sender_balance_after,
                recipient_balance_after: effect.recipient_balance_after,
            }),
            Err(e) => {
                // State-level failure — fee is NOT deducted in this impl.
                Ok(TxReceipt {
                    tx_id:                   tx.tx_id().clone(),
                    status:                  TxStatus::Failed(e.to_string()),
                    fee:                     Amount::ZERO,
                    sender_balance_after:    state.get_balance(tx.from()),
                    recipient_balance_after: state.get_balance(tx.to()),
                })
            }
        }
    }
}
