// lib.rs — Public API for the genesis crate.
//
//   use genesis::{GenesisLoader, GenesisConfig, GenesisBlock, GenesisState};
//   use genesis::GenesisError;
//
// Typical usage in the node:
//
//   let config       = GenesisLoader::from_file("config/genesis.toml")?;
//   let world_state  = GenesisState::build(&config)?;
//   let state_root   = world_state.state_root().clone();
//   let genesis_block = GenesisBlock::build(&config, state_root)?;
//   // Now pass both to Blockchain::new() and Executor::new()

mod error;
mod loader;
mod genesis_state;
mod genesis_block;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use error::GenesisError;
pub use loader::{GenesisConfig, GenesisAccount, GenesisLoader};
pub use genesis_state::GenesisState;
pub use genesis_block::{GenesisBlock, genesis_parent_hash};

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use primitives::constants::{CHAIN_ID, MAX_SUPPLY_MICRO};
    use primitives::micro_to_display;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn alice_address() -> String {
        // A valid deterministic address for tests.
        let kp = crypto::KeyPair::generate().unwrap();
        crypto::Address::from_public_key(kp.public_key()).to_checksum_hex()
    }

    fn config_with_accounts(accounts: Vec<GenesisAccount>) -> GenesisConfig {
        let mut cfg = GenesisLoader::default_config();
        cfg.initial_accounts = accounts;
        cfg
    }

    // ── GenesisLoader ─────────────────────────────────────────────────────────

    #[test]
    fn test_default_config_is_valid() {
        let cfg = GenesisLoader::default_config();
        assert!(cfg.validate().is_ok());
        assert_eq!(cfg.chain_id, CHAIN_ID);
        assert!(cfg.initial_difficulty >= 1);
    }

    #[test]
    fn test_from_str_valid_toml() {
        let toml = format!(r#"
chain_id           = {chain_id}
chain_name         = "Sypcoin"
initial_difficulty = 1
block_time_ms      = 10000
timestamp          = 1700000000000
initial_accounts   = []
"#, chain_id = CHAIN_ID);

        let cfg = GenesisLoader::from_str(&toml).unwrap();
        assert_eq!(cfg.chain_id, CHAIN_ID);
    }

    #[test]
    fn test_from_str_wrong_chain_id() {
        let toml = r#"
chain_id           = 9999
chain_name         = "OTHER"
initial_difficulty = 1
block_time_ms      = 10000
timestamp          = 1700000000000
initial_accounts   = []
"#;
        let result = GenesisLoader::from_str(toml);
        assert!(matches!(result, Err(GenesisError::InvalidChainId { .. })));
    }

    #[test]
    fn test_from_str_zero_difficulty() {
        let toml = format!(r#"
chain_id           = {chain_id}
chain_name         = "Sypcoin"
initial_difficulty = 0
block_time_ms      = 10000
timestamp          = 1700000000000
initial_accounts   = []
"#, chain_id = CHAIN_ID);

        let result = GenesisLoader::from_str(&toml);
        assert!(matches!(result, Err(GenesisError::InvalidDifficulty)));
    }

    #[test]
    fn test_to_toml_roundtrip() {
        let cfg1 = GenesisLoader::default_config();
        let toml  = GenesisLoader::to_toml(&cfg1).unwrap();
        let cfg2  = GenesisLoader::from_str(&toml).unwrap();
        assert_eq!(cfg1.chain_id,           cfg2.chain_id);
        assert_eq!(cfg1.initial_difficulty, cfg2.initial_difficulty);
        assert_eq!(cfg1.timestamp,          cfg2.timestamp);
    }

    // ── GenesisState ─────────────────────────────────────────────────────────

    #[test]
    fn test_genesis_state_no_accounts() {
        let cfg   = GenesisLoader::default_config();
        let state = GenesisState::build(&cfg).unwrap();
        assert_eq!(state.account_count(), 0);
        assert!(state.total_supply().is_zero());
    }

    #[test]
    fn test_genesis_state_with_accounts() {
        let addr = alice_address();
        let cfg  = config_with_accounts(vec![
            GenesisAccount {
                address: addr.clone(),
                balance: "1000.000000".into(),
                label:   Some("Alice".into()),
            }
        ]);

        let state = GenesisState::build(&cfg).unwrap();
        assert_eq!(state.account_count(), 1);

        let parsed_addr = crypto::Address::from_checksum_hex(&addr).unwrap();
        assert_eq!(
            state.get_balance(&parsed_addr).as_micro(),
            1_000 * 1_000_000
        );
        assert_eq!(state.total_supply().as_micro(), 1_000 * 1_000_000);
    }

    #[test]
    fn test_genesis_state_multiple_accounts() {
        let addr1 = alice_address();
        let addr2 = alice_address(); // different key each time

        let cfg = config_with_accounts(vec![
            GenesisAccount { address: addr1.clone(), balance: "500.000000".into(), label: None },
            GenesisAccount { address: addr2.clone(), balance: "300.000000".into(), label: None },
        ]);

        let state = GenesisState::build(&cfg).unwrap();
        assert_eq!(state.account_count(), 2);
        assert_eq!(state.total_supply().as_micro(), 800 * 1_000_000);
        assert!(state.verify_supply_invariant());
    }

    #[test]
    fn test_genesis_state_invalid_address() {
        let cfg = config_with_accounts(vec![
            GenesisAccount {
                address: "0xinvalidaddress".into(),
                balance: "100.000000".into(),
                label:   None,
            }
        ]);
        let result = GenesisState::build(&cfg);
        assert!(matches!(result, Err(GenesisError::InvalidAccount { .. })));
    }

    #[test]
    fn test_genesis_state_invalid_balance() {
        let addr = alice_address();
        let cfg  = config_with_accounts(vec![
            GenesisAccount {
                address: addr,
                balance: "notanumber".into(),
                label:   None,
            }
        ]);
        let result = GenesisState::build(&cfg);
        assert!(matches!(result, Err(GenesisError::InvalidAccount { .. })));
    }

    #[test]
    fn test_genesis_state_supply_exceeded() {
        let addr = alice_address();
        // MAX_SUPPLY + 1 micro-token.
        let over_max = micro_to_display(MAX_SUPPLY_MICRO + 1);
        let cfg = config_with_accounts(vec![
            GenesisAccount {
                address: addr,
                balance: over_max,
                label:   None,
            }
        ]);
        let result = GenesisState::build(&cfg);
        assert!(matches!(result,
            Err(GenesisError::SupplyExceeded { .. }) |
            Err(GenesisError::InvalidAccount { .. })
        ));
    }

    #[test]
    fn test_genesis_state_deterministic() {
        let addr = alice_address();
        let cfg  = config_with_accounts(vec![
            GenesisAccount { address: addr, balance: "42.000000".into(), label: None }
        ]);

        let s1 = GenesisState::build(&cfg).unwrap();
        let s2 = GenesisState::build(&cfg).unwrap();

        // Same config → same state root.
        assert_eq!(
            s1.state_root().as_bytes(),
            s2.state_root().as_bytes()
        );
    }

    // ── GenesisBlock ─────────────────────────────────────────────────────────

    #[test]
    fn test_genesis_block_height_zero() {
        let cfg        = GenesisLoader::default_config();
        let state      = GenesisState::build(&cfg).unwrap();
        let root       = state.state_root().clone();
        let block      = GenesisBlock::build(&cfg, root).unwrap();

        assert!(block.is_genesis());
        assert_eq!(block.height().as_u64(), 0);
    }

    #[test]
    fn test_genesis_block_parent_hash() {
        let cfg   = GenesisLoader::default_config();
        let state = GenesisState::build(&cfg).unwrap();
        let root  = state.state_root().clone();
        let block = GenesisBlock::build(&cfg, root).unwrap();

        let expected_parent = genesis_parent_hash();
        assert_eq!(
            block.parent_hash().as_bytes(),
            expected_parent.as_bytes()
        );
    }

    #[test]
    fn test_genesis_block_difficulty() {
        let mut cfg     = GenesisLoader::default_config();
        cfg.initial_difficulty = 42;

        let state = GenesisState::build(&cfg).unwrap();
        let root  = state.state_root().clone();
        let block = GenesisBlock::build(&cfg, root).unwrap();

        assert_eq!(block.difficulty(), 42);
    }

    #[test]
    fn test_genesis_block_timestamp() {
        let cfg   = GenesisLoader::default_config();
        let state = GenesisState::build(&cfg).unwrap();
        let root  = state.state_root().clone();
        let block = GenesisBlock::build(&cfg, root).unwrap();

        assert_eq!(block.timestamp().as_millis(), cfg.timestamp);
    }

    #[test]
    fn test_genesis_block_no_transactions() {
        let cfg   = GenesisLoader::default_config();
        let state = GenesisState::build(&cfg).unwrap();
        let root  = state.state_root().clone();
        let block = GenesisBlock::build(&cfg, root).unwrap();

        assert_eq!(block.tx_count(), 0);
    }

    #[test]
    fn test_genesis_parent_hash_is_constant() {
        // Must be deterministic — same value on every call.
        let h1 = genesis_parent_hash();
        let h2 = genesis_parent_hash();
        assert_eq!(h1.as_bytes(), h2.as_bytes());
    }

    // ── Full pipeline ─────────────────────────────────────────────────────────

    #[test]
    fn test_full_genesis_pipeline() {
        let addr = alice_address();
        let cfg  = config_with_accounts(vec![
            GenesisAccount {
                address: addr.clone(),
                balance: "21000000.000000".into(), // max supply
                label:   Some("foundation".into()),
            }
        ]);

        let state = GenesisState::build(&cfg).unwrap();
        assert!(state.verify_supply_invariant());

        let root  = state.state_root().clone();
        let block = GenesisBlock::build(&cfg, root.clone()).unwrap();

        assert!(block.is_genesis());
        assert_eq!(block.state_root().as_bytes(), root.as_bytes());
        assert_eq!(block.tx_count(), 0);
        assert!(block.difficulty() >= 1);
    }
}
