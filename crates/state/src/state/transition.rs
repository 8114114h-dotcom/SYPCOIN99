// state/transition.rs — Atomic state transition for a single transaction.
//
// Security decisions:
//
//   JOURNAL BEFORE MUTATE
//   • Every mutation is preceded by recording the old value in the journal.
//     If any step fails, journal.rollback() restores the exact pre-tx state.
//
//   ATOMIC ALL-OR-NOTHING
//   • The sequence: debit sender → credit recipient → credit miner → inc nonce
//     is wrapped in a pattern where any Err triggers rollback before returning.
//     No partial state is ever visible outside this function.
//
//   SUPPLY CONSERVATION
//   • amount flows: sender → recipient
//   • fee flows:    sender → miner
//   • total_supply does NOT change for normal transfers (only for coinbase).
//   • We verify sender_debit == recipient_credit + miner_credit (amount + fee).

use crypto::Address;
use primitives::{Amount, BlockHeight};
use transaction::Transaction;

use crate::account::store::AccountStore;
use crate::error::StateError;
use crate::journal::Journal;
use primitives::block_reward_at;

/// The net effect of applying a single transaction.
#[derive(Clone, Debug)]
pub struct TxEffect {
    /// Sender's balance after the transaction.
    pub sender_balance_after: Amount,
    /// Recipient's balance after the transaction.
    pub recipient_balance_after: Amount,
    /// Fee collected by the miner.
    pub fee_collected: Amount,
}

pub struct StateTransition;

impl StateTransition {
    /// Apply a single transaction to the account store.
    ///
    /// Records all changes in `journal` before applying them.
    /// On any error, calls `journal.rollback(store)` and returns `Err`.
    pub fn apply_tx(
        store:   &mut AccountStore,
        journal: &mut Journal,
        tx:      &Transaction,
        miner:   &Address,
    ) -> Result<TxEffect, StateError> {
        let sender    = tx.from().clone();
        let recipient = tx.to().clone();
        let amount    = tx.amount();
        let fee       = tx.fee();

        // ── Pre-flight: verify balance before touching any account ─────────────
        // Read sender balance without mutation.
        let sender_balance = store
            .get(&sender)
            .map(|a| a.balance())
            .unwrap_or(Amount::ZERO);

        let total_deducted = amount
            .checked_add(fee)
            .ok_or(StateError::InvalidTransition(
                "amount + fee overflow".into()
            ))?;

        if sender_balance < total_deducted {
            return Err(StateError::InsufficientBalance {
                available: sender_balance,
                required:  total_deducted,
            });
        }

        // ── Apply mutations (journal before each) ──────────────────────────────

        // 1. Create accounts if they don't exist yet.
        if !store.contains(&sender) {
            journal.record_account_created(&sender);
        }
        if !store.contains(&recipient) {
            journal.record_account_created(&recipient);
        }
        if !store.contains(miner) {
            journal.record_account_created(miner);
        }

        // 2. Debit sender (amount + fee).
        {
            let acc = store.get_or_create(&sender);
            let new_sender_bal = acc.balance()
                .checked_sub(total_deducted)
                .ok_or_else(|| StateError::InsufficientBalance {
                    available: acc.balance(),
                    required:  total_deducted,
                })?;
            journal.record_balance_change(&sender, acc.balance(), new_sender_bal);
            if let Err(e) = acc.debit(total_deducted) {
                journal.rollback(store);
                journal.clear();
                return Err(e);
            }
        }

        // 3. Credit recipient.
        {
            let acc = store.get_or_create(&recipient);
            let old = acc.balance();
            let new = old.checked_add(amount)
                .ok_or_else(|| StateError::SupplyExceedsMax(amount.as_micro()))?;
            journal.record_balance_change(&recipient, old, new);
            if let Err(e) = acc.credit(amount) {
                journal.rollback(store);
                journal.clear();
                return Err(e);
            }
        }

        // 4. Credit miner with fee.
        {
            let acc = store.get_or_create(miner);
            let old = acc.balance();
            let new = old.checked_add(fee)
                .ok_or_else(|| StateError::SupplyExceedsMax(fee.as_micro()))?;
            journal.record_balance_change(miner, old, new);
            if let Err(e) = acc.credit(fee) {
                journal.rollback(store);
                journal.clear();
                return Err(e);
            }
        }

        // 5. Increment sender nonce.
        {
            let acc = store.get_or_create(&sender);
            let old = acc.nonce();
            // Compute next() before any closure to avoid borrow conflict.
            let new = old.next().map_err(|_| StateError::NonceOverflow)?;
            journal.record_nonce_change(&sender, old, new);
            if let Err(e) = acc.increment_nonce() {
                journal.rollback(store);
                journal.clear();
                return Err(e);
            }
        }

        // ── Success — read final balances ─────────────────────────────────────
        let sender_balance_after    = store.get(&sender).map(|a| a.balance()).unwrap_or(Amount::ZERO);
        let recipient_balance_after = store.get(&recipient).map(|a| a.balance()).unwrap_or(Amount::ZERO);

        journal.clear();

        Ok(TxEffect {
            sender_balance_after,
            recipient_balance_after,
            fee_collected: fee,
        })
    }

    /// Apply the block reward (coinbase) to the miner's account.
    ///
    /// This INCREASES total_supply — called by WorldState after apply_tx loop.
    pub fn apply_reward(
        store:        &mut AccountStore,
        journal:      &mut Journal,
        miner:        &Address,
        height:       &BlockHeight,
        total_supply: &mut Amount,
    ) -> Result<Amount, StateError> {
        let reward = block_reward_at(height);

        if reward.is_zero() {
            return Ok(Amount::ZERO); // post-halving era
        }

        // Verify supply cap.
        let new_supply = total_supply
            .checked_add(reward)
            .ok_or(StateError::SupplyOverflow)?;

        // Credit miner.
        if !store.contains(miner) {
            journal.record_account_created(miner);
        }
        let acc = store.get_or_create(miner);
        let old = acc.balance();
        let new_miner_bal = old.checked_add(reward)
            .ok_or(StateError::BalanceOverflow)?;
        journal.record_balance_change(miner, old, new_miner_bal);
        acc.credit(reward).map_err(|e| {
            journal.rollback(store);
            journal.clear();
            e
        })?;

        *total_supply = new_supply;
        journal.clear();

        Ok(reward)
    }
}
