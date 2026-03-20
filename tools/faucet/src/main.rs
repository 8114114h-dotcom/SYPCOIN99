// faucet/main.rs — MYCHAIN testnet faucet.
//
// Usage:
//   sypcoin-faucet --to 0xAddress --amount 10 --rpc http://127.0.0.1:8545
//
// The faucet holds a pre-funded keypair (loaded from env FAUCET_SEED or
// generated deterministically). It builds a signed transaction and submits
// it via the node's RPC sendTransaction endpoint.
//
// IMPORTANT: Only run on testnet/devnet. Never on mainnet.

use crypto::{Address, KeyPair};
use primitives::{Amount, Nonce};
use primitives::constants::MIN_TX_FEE_MICRO;
use transaction::TransactionBuilder;

/// Maximum tokens dispensable per request.
const MAX_DRIP_TOKENS: u64 = 100;
/// Default drip amount in tokens.
const DEFAULT_DRIP_TOKENS: u64 = 10;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() || args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return;
    }

    let rpc_url    = flag_value(&args, "--rpc").unwrap_or("http://127.0.0.1:8545".into());
    let to_str     = match flag_value(&args, "--to") {
        Some(a) => a,
        None    => { eprintln!("[ERROR] --to <address> is required"); std::process::exit(1); }
    };
    let amount_tok = flag_value(&args, "--amount")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(DEFAULT_DRIP_TOKENS);
    let nonce_val  = flag_value(&args, "--nonce")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1);

    println!("╔═══════════════════════════════════════════════╗");
    println!("║         Sypcoin Testnet Faucet                ║");
    println!("╚═══════════════════════════════════════════════╝");

    // Validate recipient address.
    let to_addr = match Address::from_checksum_hex(&to_str) {
        Ok(a)  => a,
        Err(_) => {
            eprintln!("[ERROR] Invalid recipient address: {}", to_str);
            std::process::exit(1);
        }
    };

    // Cap the drip amount.
    let amount_tok = amount_tok.min(MAX_DRIP_TOKENS);
    let amount     = match Amount::from_tokens(amount_tok) {
        Ok(a)  => a,
        Err(e) => { eprintln!("[ERROR] {}", e); std::process::exit(1); }
    };

    println!("  RPC           : {}", rpc_url);
    println!("  Recipient     : {}", to_addr.to_checksum_hex());
    println!("  Amount        : {} tokens", amount_tok);

    // Load or generate faucet keypair.
    // In production: load from encrypted keystore file.
    // For demo: use a fixed seed.
    let faucet_kp = load_faucet_keypair();
    let faucet_addr = Address::from_public_key(faucet_kp.public_key());
    println!("  Faucet addr   : {}", faucet_addr.to_checksum_hex());

    // Build and sign transaction.
    let fee   = Amount::from_micro(MIN_TX_FEE_MICRO).unwrap();
    let nonce = Nonce::new(nonce_val);

    let tx = match TransactionBuilder::new()
        .from_keypair(faucet_kp)
        .to(to_addr)
        .amount(amount)
        .fee(fee)
        .nonce(nonce)
        .build()
    {
        Ok(t)  => t,
        Err(e) => { eprintln!("[ERROR] Transaction build failed: {}", e); std::process::exit(1); }
    };

    let tx_id  = hex::encode(tx.tx_id().as_bytes());
    let tx_hex = hex::encode(bincode::serialize(&tx).unwrap());

    println!("  TX ID         : {}", &tx_id[..16]);
    println!();

    // Submit via RPC (simulated — real HTTP call would go here).
    println!("  Submitting transaction...");
    println!("  [DEMO] Would POST to: {}/sendTransaction", rpc_url);
    println!("  [DEMO] tx_hex length: {} bytes", tx_hex.len() / 2);
    println!();
    println!("  ✓ Transaction ready!");
    println!("  TX ID  : {}", tx_id);
    println!();
    println!("  To actually submit, run:");
    println!("    curl -X POST {} \\", rpc_url);
    println!("      -H 'Content-Type: application/json' \\");
    println!("      -d '{{\"jsonrpc\":\"2.0\",\"method\":\"sendTransaction\",\"params\":[\"{}\"],\"id\":1}}'",
             &tx_hex[..32]);
}

fn load_faucet_keypair() -> KeyPair {
    // In production: load from FAUCET_KEYSTORE env var pointing to an
    // encrypted keystore file. For demo: fixed seed.
    let seed_hex = std::env::var("FAUCET_SEED")
        .unwrap_or_else(|_| "0".repeat(64));

    let seed_bytes = hex::decode(&seed_hex)
        .unwrap_or_else(|_| vec![0u8; 32]);

    let mut seed = [0u8; 32];
    let len = seed_bytes.len().min(32);
    seed[..len].copy_from_slice(&seed_bytes[..len]);

    // Use from_seed (test-utils feature).
    crypto::KeyPair::from_seed(seed)
        .unwrap_or_else(|_| KeyPair::generate().unwrap())
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|w| w[0] == flag)
        .map(|w| w[1].clone())
}

fn print_help() {
    println!(r#"
Sypcoin Testnet Faucet

USAGE:
    sypcoin-faucet --to <ADDRESS> [OPTIONS]

OPTIONS:
    --to      <ADDRESS>   Recipient address (required)
    --amount  <TOKENS>    Amount to send in tokens (default: 10, max: 100)
    --nonce   <N>         Faucet account nonce (default: 1)
    --rpc     <URL>       Node RPC URL (default: http://127.0.0.1:8545)

ENVIRONMENT:
    FAUCET_SEED   Hex-encoded 32-byte seed for faucet keypair

WARNING: Only use on testnet/devnet. Never on mainnet.
"#);
}
