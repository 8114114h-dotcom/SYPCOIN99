// account/account.rs — The Account type.
//
// Security decisions:
//
//   ALL MUTATIONS ARE CHECKED
//   • credit() and debit() use checked arithmetic.
//   • debit() verifies balance ≥ amount before subtracting — no underflow.
//   • credit() verifies the result would not exceed MAX_SUPPLY_MICRO.
//     (The state layer enforces the global supply cap separately via
//     WorldState::total_supply, but per-account we still bound it.)
//
//   NONCE IS MONOTONIC
//   • increment_nonce() uses Nonce::next() which returns Err on overflow.
//   • Callers must not skip nonces — the transaction validator enforces
//     that each tx nonce == account.expected_nonce().
//
//   NO DEFAULT BALANCE
//   • Account::new() always starts with balance=0, nonce=0.
//     There is no way to construct an Account with an arbitrary balance
//     except through credit() — which is always journaled.

use serde::{Deserialize, Serialize};

use crypto::Address;
use primitives::{Amount, Nonce};
use primitives::constants::MAX_SUPPLY_MICRO;

use crate::error::StateError;

/// A single account in the world state.
///
/// Tracks balance (in micro-tokens) and nonce (transaction counter).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    /// The account's address (derived from its public key).
    pub(crate) address: Address,

    /// Current balance in micro-tokens.
    pub(crate) balance: Amount,

    /// Number of confirmed transactions sent from this account.
    pub(crate) nonce: Nonce,
}

impl Account {
    /// Create a new account with zero balance and zero nonce.
    pub fn new(address: Address) -> Self {
        Account {
            address,
            balance: Amount::ZERO,
            nonce:   Nonce::ZERO,
        }
    }

    // ── Accessors ────────────────────────────────────────────────────────────

    pub fn address(&self) -> &Address { &self.address }
    pub fn balance(&self) -> Amount   { self.balance }
    pub fn nonce(&self)   -> Nonce    { self.nonce }

    /// The nonce value the next outgoing transaction must have.
    ///
    /// = current_nonce + 1. If current_nonce == u64::MAX, returns Err.
    pub fn expected_nonce(&self) -> Result<Nonce, StateError> {
        self.nonce.next().map_err(|_| StateError::NonceOverflow)
    }

    // ── Mutations (always journaled by the caller) ────────────────────────────

    /// Add `amount` to this account's balance.
    ///
    /// Returns `Err` if the result would exceed MAX_SUPPLY_MICRO.
    pub fn credit(&mut self, amount: Amount) -> Result<(), StateError> {
        let new_balance = self.balance
            .checked_add(amount)
            .ok_or_else(|| StateError::SupplyExceedsMax(amount.as_micro()))?;

        // Belt-and-suspenders: also check raw u64 cap.
        if new_balance.as_micro() > MAX_SUPPLY_MICRO {
            return Err(StateError::SupplyExceedsMax(amount.as_micro()));
        }

        self.balance = new_balance;
        Ok(())
    }

    /// Subtract `amount` from this account's balance.
    ///
    /// Returns `Err(InsufficientBalance)` if balance < amount.
    pub fn debit(&mut self, amount: Amount) -> Result<(), StateError> {
        let new_balance = self.balance
            .checked_sub(amount)
            .ok_or_else(|| StateError::InsufficientBalance {
                available: self.balance,
                required:  amount,
            })?;
        self.balance = new_balance;
        Ok(())
    }

    /// Increment the nonce by 1.
    ///
    /// Called after a transaction from this account is confirmed.
    pub fn increment_nonce(&mut self) -> Result<(), StateError> {
        self.nonce = self.nonce.next().map_err(|_| StateError::NonceOverflow)?;
        Ok(())
    }
}

impl std::fmt::Display for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Account[{}] balance={} nonce={}",
            self.address.to_checksum_hex(),
            self.balance,
            self.nonce,
        )
    }
}
