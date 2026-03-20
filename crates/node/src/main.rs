// main.rs — Sypcoin node entry point.
//
// Parses CLI arguments, loads config, and starts the node.

mod cli;
mod node {
    pub(crate) mod lifecycle;
    pub(crate) mod runner;
    pub(crate) mod services;
    pub(crate) mod shutdown;
}

use cli::{CliCommand, print_help, print_version};
use config::ConfigLoader;
use node::runner::NodeRunner;

fn main() {
    let command = CliCommand::parse();

    match command {
        CliCommand::Help => {
            print_help();
        }

        CliCommand::Version => {
            print_version();
        }

        CliCommand::Init { data_dir, network } => {
            cmd_init(&data_dir, &network);
        }

        CliCommand::Start { config, mine, miner_addr, network } => {
            cmd_start(config, mine, miner_addr, network);
        }
    }
}

// ── Command handlers ──────────────────────────────────────────────────────────

fn cmd_start(
    config_path:  Option<String>,
    mine:         bool,
    miner_addr:   Option<String>,
    network:      String,
) {
    // Load config.
    let mut cfg = match config_path {
        Some(path) => match ConfigLoader::from_file(&path) {
            Ok(c)  => c,
            Err(e) => { eprintln!("[ERROR] Config load failed: {}", e); std::process::exit(1); }
        },
        None => match network.as_str() {
            "testnet" => ConfigLoader::default_testnet(),
            "devnet"  => ConfigLoader::default_devnet(),
            _         => ConfigLoader::default_mainnet(),
        },
    };

    // CLI flags override config.
    if mine {
        cfg.consensus.mine_on_start = true;
    }
    if let Some(ref addr) = miner_addr {
        cfg.consensus.miner_address = Some(addr.clone());
    }

    // Apply environment variable overrides.
    ConfigLoader::apply_env(&mut cfg);

    // Build and start the node.
    let effective_miner = miner_addr.or_else(|| cfg.consensus.miner_address.clone());
    let mine_flag       = mine || cfg.consensus.mine_on_start;

    let mut runner = match NodeRunner::new(cfg, mine_flag, effective_miner) {
        Ok(r)  => r,
        Err(e) => { eprintln!("[ERROR] Node init failed: {}", e); std::process::exit(1); }
    };

    // Install SIGINT handler (Ctrl+C → graceful shutdown).
    // In production, use ctrlc crate. Here we use a simple flag.
    let shutdown_clone = runner.services.network.connected_count(); // just to touch services

    runner.start();
    runner.run();

    eprintln!("[INFO] Node exited cleanly.");
}

fn cmd_init(data_dir: &str, network: &str) {
    use std::fs;

    let config_dir = format!("{}/config", data_dir);
    fs::create_dir_all(&config_dir)
        .unwrap_or_else(|e| { eprintln!("[ERROR] {}", e); std::process::exit(1); });

    // Write default config file.
    let cfg = match network {
        "testnet" => ConfigLoader::default_testnet(),
        "devnet"  => ConfigLoader::default_devnet(),
        _         => ConfigLoader::default_mainnet(),
    };

    let toml = ConfigLoader::to_toml(&cfg)
        .unwrap_or_else(|e| { eprintln!("[ERROR] {}", e); std::process::exit(1); });

    let config_path = format!("{}/node.toml", config_dir);
    fs::write(&config_path, toml)
        .unwrap_or_else(|e| { eprintln!("[ERROR] {}", e); std::process::exit(1); });

    println!("[OK] Initialised {} node in '{}'", network, data_dir);
    println!("[OK] Config written to '{}'", config_path);
    println!("[OK] Run: sypcoin start --config {}", config_path);
}
