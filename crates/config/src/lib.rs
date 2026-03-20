// lib.rs — Public API for the config crate.
//
//   use config::{NodeConfig, ConfigLoader, NetworkConfig, ConsensusConfig};
//   use config::{StorageConfig, RpcConfig, DbBackend, ConfigError};

mod error;
mod network;
mod consensus;
mod storage;
mod loader;

pub use error::ConfigError;
pub use network::NetworkConfig;
pub use consensus::ConsensusConfig;
pub use storage::{StorageConfig, DbBackend};
pub use loader::{NodeConfig, RpcConfig, ConfigLoader};

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Default configs ───────────────────────────────────────────────────────

    #[test]
    fn test_default_mainnet_is_valid() {
        let cfg = ConfigLoader::default_mainnet();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_default_testnet_is_valid() {
        let cfg = ConfigLoader::default_testnet();
        assert!(cfg.validate().is_ok());
        assert_eq!(cfg.consensus.initial_difficulty, 100);
    }

    #[test]
    fn test_default_devnet_is_valid() {
        let cfg = ConfigLoader::default_devnet();
        assert!(cfg.validate().is_ok());
        assert_eq!(cfg.consensus.initial_difficulty, 1);
        assert_eq!(cfg.storage.db_backend, DbBackend::InMemory);
        assert_eq!(cfg.log_level, "debug");
    }

    // ── TOML parsing ──────────────────────────────────────────────────────────

    #[test]
    fn test_from_str_minimal_valid() {
        let toml = r#"
[network]
listen_addr = "0.0.0.0:30303"

[consensus]
initial_difficulty = 1000

[storage]
data_dir   = "./data"
db_backend = "rocksdb"

[rpc]
enabled     = true
listen_addr = "127.0.0.1:8545"

log_level = "info"
"#;
        let cfg = ConfigLoader::from_str(toml).unwrap();
        assert_eq!(cfg.consensus.initial_difficulty, 1000);
        assert_eq!(cfg.log_level, "info");
        assert!(cfg.rpc.enabled);
    }

    #[test]
    fn test_from_str_defaults_applied() {
        // Minimal TOML — all defaults should kick in.
        let toml = r#"log_level = "info""#;
        let cfg  = ConfigLoader::from_str(toml).unwrap();
        assert_eq!(cfg.network.max_inbound, 32);
        assert_eq!(cfg.storage.snapshots_to_keep, 20);
    }

    #[test]
    fn test_from_str_invalid_toml() {
        let result = ConfigLoader::from_str("[[[[not valid toml");
        assert!(matches!(result, Err(ConfigError::ParseError(_))));
    }

    #[test]
    fn test_to_toml_roundtrip() {
        let cfg1  = ConfigLoader::default_mainnet();
        let toml  = ConfigLoader::to_toml(&cfg1).unwrap();
        let cfg2  = ConfigLoader::from_str(&toml).unwrap();
        assert_eq!(cfg1.consensus.initial_difficulty,
                   cfg2.consensus.initial_difficulty);
        assert_eq!(cfg1.network.max_peers, cfg2.network.max_peers);
        assert_eq!(cfg1.log_level,         cfg2.log_level);
    }

    // ── NetworkConfig validation ───────────────────────────────────────────────

    #[test]
    fn test_network_invalid_peer_count() {
        let mut net = NetworkConfig::default();
        net.max_inbound  = 100;
        net.max_outbound = 100;
        net.max_peers    = 50; // less than inbound+outbound
        assert!(net.validate().is_err());
    }

    #[test]
    fn test_network_zero_ping_interval() {
        let mut net = NetworkConfig::default();
        net.ping_interval_ms = 0;
        assert!(net.validate().is_err());
    }

    #[test]
    fn test_network_bootstrap_peers() {
        let mut cfg = ConfigLoader::default_mainnet();
        cfg.network.bootstrap_peers = vec![
            "192.168.1.1:30303".into(),
            "10.0.0.1:30303".into(),
        ];
        assert!(cfg.validate().is_ok());
        assert_eq!(cfg.network.bootstrap_peers.len(), 2);
    }

    // ── ConsensusConfig validation ────────────────────────────────────────────

    #[test]
    fn test_consensus_zero_difficulty() {
        let mut cons = ConsensusConfig::default();
        cons.initial_difficulty = 0;
        assert!(cons.validate().is_err());
    }

    #[test]
    fn test_consensus_mine_without_address() {
        let mut cons = ConsensusConfig::default();
        cons.mine_on_start = true;
        cons.miner_address = None;
        assert!(cons.validate().is_err());
    }

    #[test]
    fn test_consensus_mine_with_address() {
        let mut cons = ConsensusConfig::default();
        cons.mine_on_start = true;
        cons.miner_address = Some("0xAbCdEf0123456789AbCdEf0123456789AbCdEf01".into());
        assert!(cons.validate().is_ok());
    }

    // ── StorageConfig validation ──────────────────────────────────────────────

    #[test]
    fn test_storage_zero_snapshot_interval() {
        let mut stor = StorageConfig::default();
        stor.snapshot_interval = 0;
        assert!(stor.validate().is_err());
    }

    #[test]
    fn test_storage_rocksdb_needs_data_dir() {
        let mut stor = StorageConfig::default();
        stor.db_backend = DbBackend::RocksDb;
        stor.data_dir   = String::new();
        assert!(stor.validate().is_err());
    }

    #[test]
    fn test_storage_inmemory_no_data_dir_needed() {
        let mut stor = StorageConfig::default();
        stor.db_backend = DbBackend::InMemory;
        stor.data_dir   = String::new();
        assert!(stor.validate().is_ok());
    }

    // ── RpcConfig validation ──────────────────────────────────────────────────

    #[test]
    fn test_rpc_enabled_needs_addr() {
        let mut rpc = RpcConfig::default();
        rpc.enabled     = true;
        rpc.listen_addr = String::new();
        assert!(rpc.validate().is_err());
    }

    #[test]
    fn test_rpc_disabled_no_addr_ok() {
        let mut rpc = RpcConfig::default();
        rpc.enabled     = false;
        rpc.listen_addr = String::new();
        assert!(rpc.validate().is_ok());
    }

    // ── Log level validation ──────────────────────────────────────────────────

    #[test]
    fn test_invalid_log_level() {
        let toml = r#"log_level = "verbose""#;
        let result = ConfigLoader::from_str(toml);
        assert!(matches!(result, Err(ConfigError::InvalidValue { .. })));
    }

    #[test]
    fn test_valid_log_levels() {
        for level in ["error", "warn", "info", "debug", "trace"] {
            let toml = format!(r#"log_level = "{}""#, level);
            assert!(ConfigLoader::from_str(&toml).is_ok(), "level '{}' should be valid", level);
        }
    }

    // ── Environment variable overrides ────────────────────────────────────────

    #[test]
    fn test_apply_env_overrides() {
        std::env::set_var("SYPCOIN_LOG_LEVEL", "debug");
        std::env::set_var("SYPCOIN_DATA_DIR",  "/tmp/mychain");

        let mut cfg = ConfigLoader::default_mainnet();
        ConfigLoader::apply_env(&mut cfg);

        assert_eq!(cfg.log_level,        "debug");
        assert_eq!(cfg.storage.data_dir, "/tmp/mychain");

        // Clean up.
        std::env::remove_var("SYPCOIN_LOG_LEVEL");
        std::env::remove_var("SYPCOIN_DATA_DIR");
    }
}
