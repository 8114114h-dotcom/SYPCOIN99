// cli.rs — Command-line interface for the Sypcoin node.
//
// Parses std::env::args() without an external crate (clap/structopt)
// to keep dependencies minimal on mobile/Termux environments.

/// Parsed CLI command.
#[derive(Debug)]
pub enum CliCommand {
    /// Start the node.
    Start {
        /// Path to the node config file (default: ./config/node.toml).
        config:      Option<String>,
        /// Enable mining on startup.
        mine:        bool,
        /// Mining reward address (required if --mine).
        miner_addr:  Option<String>,
        /// Network: mainnet | testnet | devnet (default: mainnet).
        network:     String,
    },

    /// Initialise a new data directory with genesis and default config.
    Init {
        data_dir: String,
        network:  String,
    },

    /// Print node version information and exit.
    Version,

    /// Print help text and exit.
    Help,
}

impl CliCommand {
    /// Parse from std::env::args().
    pub fn parse() -> Self {
        let args: Vec<String> = std::env::args().skip(1).collect();
        Self::parse_args(&args)
    }

    /// Parse from a slice of string arguments (testable).
    pub fn parse_args(args: &[String]) -> Self {
        if args.is_empty() {
            return CliCommand::Help;
        }

        match args[0].as_str() {
            "start" | "run" => {
                let mut config     = None;
                let mut mine       = false;
                let mut miner_addr = None;
                let mut network    = "mainnet".to_owned();

                let mut i = 1;
                while i < args.len() {
                    match args[i].as_str() {
                        "--config" | "-c" => {
                            if i + 1 < args.len() { config = Some(args[i + 1].clone()); i += 1; }
                        }
                        "--mine" | "-m" => { mine = true; }
                        "--miner" => {
                            if i + 1 < args.len() { miner_addr = Some(args[i + 1].clone()); i += 1; }
                        }
                        "--network" | "-n" => {
                            if i + 1 < args.len() { network = args[i + 1].clone(); i += 1; }
                        }
                        _ => {}
                    }
                    i += 1;
                }

                CliCommand::Start { config, mine, miner_addr, network }
            }

            "init" => {
                let data_dir = args.get(1).cloned().unwrap_or_else(|| "./data".into());
                let network  = args.get(2).cloned().unwrap_or_else(|| "mainnet".into());
                CliCommand::Init { data_dir, network }
            }

            "version" | "--version" | "-V" => CliCommand::Version,

            _ => CliCommand::Help,
        }
    }
}

/// Print help text to stdout.
pub fn print_help() {
    println!("{}", HELP_TEXT);
}

/// Print version information to stdout.
pub fn print_version() {
    println!("mychain {}", env!("CARGO_PKG_VERSION"));
    println!("Layer 1 Blockchain Node");
}

const HELP_TEXT: &str = r#"
Sypcoin Node

USAGE:
    mychain <COMMAND> [OPTIONS]

COMMANDS:
    start       Start the node
    init        Initialise a new data directory
    version     Print version information

START OPTIONS:
    --config, -c <PATH>     Config file path (default: ./config/node.toml)
    --mine,   -m            Enable mining on startup
    --miner   <ADDRESS>     Mining reward address (required with --mine)
    --network,-n <NET>      Network: mainnet | testnet | devnet

INIT OPTIONS:
    <DATA_DIR>              Directory to initialise (default: ./data)
    <NETWORK>               Network preset (default: mainnet)

EXAMPLES:
    sypcoin start
    sypcoin start --mine --miner 0xYourAddress
    sypcoin start --config /etc/mychain/node.toml --network testnet
    sypcoin init ./mydata devnet
    sypcoin version
"#;
