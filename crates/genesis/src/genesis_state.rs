// genesis_state.rs — Build the initial world state from genesis config.
//
// Security decisions:
//   • Every account address is validated via Address::from_checksum_hex.
//     An invalid address in genesis.toml is a hard error — the chain
//     cannot start with corrupted accounts.
//
//   • Every balance is parsed via display_to_micro which validates the
//     decimal format and rejects values > MAX_SUPPLY_MICRO.
//
//   • Total supply is checked against MAX_SUPPLY_MICRO before applying
//     any balance to the state. This prevents overflow attacks via
//     crafted genesis configs.
//
//   • state.commit() is called after all accounts are seeded, producing
//     the state_root used in the genesis block header. This root commits
//     to all initial balances and must be identical on every node.

use crypto::Address;
use primitives::{Amount, BlockHeight, display_to_micro};
use primitives::constants::MAX_SUPPLY_MICRO;
use state::WorldState;

use crate::error::GenesisError;
use crate::loader::GenesisConfig;

pub struct GenesisState;

impl GenesisState {
    /// Build and return the genesis WorldState from config.
    ///
    /// Returns `(world_state, state_root)` where `state_root` is the
    /// Merkle root after all genesis accounts are seeded.
    pub fn build(
        config: &GenesisConfig,
    ) -> Result<WorldState, GenesisError> {
        let mut state = WorldState::new();

        // ── 1. Parse and validate all accounts ────────────────────────────────
        let mut total_micro: u64 = 0;

        for entry in &config.initial_accounts {
            // Validate address.
            let addr = Address::from_checksum_hex(&entry.address)
                .map_err(|_| GenesisError::InvalidAccount {
                    address: entry.address.clone(),
                    reason:  "invalid checksum hex address".into(),
                })?;

            // Parse balance.
            let micro = display_to_micro(&entry.balance)
                .map_err(|e| GenesisError::InvalidAccount {
                    address: entry.address.clone(),
                    reason:  format!("invalid balance '{}': {}", entry.balance, e),
                })?;

            // Accumulate and check supply cap.
            total_micro = total_micro
                .checked_add(micro)
                .ok_or(GenesisError::SupplyExceeded {
                    total: u64::MAX,
                    max:   MAX_SUPPLY_MICRO,
                })?;

            if total_micro > MAX_SUPPLY_MICRO {
                return Err(GenesisError::SupplyExceeded {
                    total: total_micro,
                    max:   MAX_SUPPLY_MICRO,
                });
            }

            // ── 2. Apply to state ──────────────────────────────────────────────
            let amount = Amount::from_micro(micro)
                .map_err(|e| GenesisError::StateError(e.to_string()))?;

            state.set_genesis_balance(addr, amount)
                .map_err(|e| GenesisError::StateError(e.to_string()))?;
        }

        // ── 3. Commit — compute and store state_root ──────────────────────────
        state.commit(BlockHeight::genesis());

        Ok(state)
    }
}
