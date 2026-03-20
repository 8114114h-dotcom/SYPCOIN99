// miner/main.rs — Standalone MYCHAIN mining tool.
//
// Usage:
//   sypcoin-miner --rpc http://127.0.0.1:8545 --address 0xYourAddress
//
// Flow per round:
//   1. Call getMiningInfo  → get difficulty + best_hash
//   2. Call getBlockTemplate → get height + parent + tx_ids
//   3. Build block template locally
//   4. Mine: find nonce where hash < target
//   5. (future) Call submitBlock → send solved block to node
//      For now: print the solved nonce and hash

use block::{BlockBuilder, difficulty_to_target, meets_target};
use consensus::Miner;
use crypto::{Address, KeyPair, sha256};
use primitives::{BlockHeight, Timestamp};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rpc_url  = flag_value(&args, "--rpc").unwrap_or("http://127.0.0.1:8545".into());
    let addr_str = flag_value(&args, "--address");
    let rounds   = flag_value(&args, "--rounds")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(u64::MAX);

    println!("╔═══════════════════════════════════════════════╗");
    println!("║         Sypcoin Standalone Miner              ║");
    println!("╚═══════════════════════════════════════════════╝");
    println!("  RPC endpoint : {}", rpc_url);

    // Derive miner address.
    let miner_addr = match addr_str {
        Some(ref a) => match Address::from_checksum_hex(a) {
            Ok(addr) => addr,
            Err(_)   => { eprintln!("[ERROR] Invalid miner address: {}", a); std::process::exit(1); }
        },
        None => {
            eprintln!("[WARN] No --address provided, generating a one-time keypair.");
            let kp = KeyPair::generate().unwrap();
            let a  = Address::from_public_key(kp.public_key());
            println!("  Generated addr: {}", a.to_checksum_hex());
            a
        }
    };

    println!("  Miner address : {}", miner_addr.to_checksum_hex());
    println!("  Rounds        : {}", if rounds == u64::MAX { "∞".into() } else { rounds.to_string() });
    println!();

    let miner = Miner::new(miner_addr.clone());
    let mut blocks_found = 0u64;

    for round in 0..rounds {
        println!("── Round {} ─────────────────────────────────────", round + 1);

        // In a real implementation, we'd call getMiningInfo and getBlockTemplate
        // via HTTP JSON-RPC. For now we simulate with local data.
        let difficulty = 1u64; // trivially easy for demo
        let height     = BlockHeight::new(round + 1);
        let parent     = sha256(&round.to_le_bytes());

        let template = match BlockBuilder::new()
            .height(height)
            .parent_hash(parent)
            .state_root(sha256(b"state"))
            .miner(miner_addr.clone())
            .difficulty(difficulty)
            .timestamp(Timestamp::now())
            .build()
        {
            Ok(t)  => t,
            Err(e) => { eprintln!("  [ERROR] Template: {}", e); continue; }
        };

        println!("  Difficulty    : {}", difficulty);
        println!("  Height        : {}", height);
        println!("  Mining...");

        let start = Timestamp::now();
        match miner.mine(template, || false) {
            Ok(result) => {
                blocks_found += 1;
                println!("  ✓ Block found!");
                println!("  Nonce         : {}", result.nonce_found);
                println!("  Hash          : {}", hex::encode(result.block.hash().as_bytes()));
                println!("  Nonces tried  : {}", result.nonces_tried);
                println!("  Elapsed       : {}ms", result.elapsed_ms);
                println!("  Hash rate     : {:.0} H/s",
                    result.nonces_tried as f64 / (result.elapsed_ms as f64 / 1000.0).max(0.001)
                );
                // TODO: submit block via RPC: POST sendBlock to rpc_url
            }
            Err(e) => {
                eprintln!("  [ERROR] Mining failed: {}", e);
            }
        }
        println!();
    }

    println!("══ Mining session complete ══");
    println!("  Blocks found  : {}", blocks_found);
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|w| w[0] == flag)
        .map(|w| w[1].clone())
}
