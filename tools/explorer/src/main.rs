// explorer/main.rs — Sypcoin CLI block explorer.
//
// Queries the node RPC and formats results for the terminal.
//
// Usage:
//   sypcoin-explorer status
//   sypcoin-explorer blocks [n]
//   sypcoin-explorer block <hash|height>
//   sypcoin-explorer tx <tx_id>
//   sypcoin-explorer address <addr>

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() || args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return;
    }

    let rpc_url = flag_value(&args, "--rpc").unwrap_or("http://127.0.0.1:8545".into());

    match args[0].as_str() {
        "status"  => cmd_status(&rpc_url),
        "blocks"  => {
            let n = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10u64);
            cmd_blocks(&rpc_url, n);
        }
        "block"   => {
            let id = args.get(1).cloned().unwrap_or_default();
            cmd_block(&rpc_url, &id);
        }
        "tx"      => {
            let tx_id = args.get(1).cloned().unwrap_or_default();
            cmd_tx(&rpc_url, &tx_id);
        }
        "address" => {
            let addr = args.get(1).cloned().unwrap_or_default();
            cmd_address(&rpc_url, &addr);
        }
        "mining"  => cmd_mining(&rpc_url),
        _         => { eprintln!("[ERROR] Unknown command: {}", args[0]); print_help(); }
    }
}

// ── Commands ──────────────────────────────────────────────────────────────────

fn cmd_status(rpc: &str) {
    println!("╔═══════════════════════════════════════════════╗");
    println!("║         Sypcoin Network Status                ║");
    println!("╚═══════════════════════════════════════════════╝");

    let height = rpc_call(rpc, "getBlockHeight", serde_json::json!([]));
    let mining = rpc_call(rpc, "getMiningInfo",  serde_json::json!([]));

    match height {
        Ok(h) => println!("  Chain height  : {}", h),
        Err(e) => println!("  Chain height  : [ERROR] {}", e),
    }

    match mining {
        Ok(m) => {
            println!("  Difficulty    : {}", m["difficulty"]);
            println!("  Best hash     : {}...", &m["best_hash"].as_str().unwrap_or("?")[..16]);
            println!("  Mining target : {}...", &m["target"].as_str().unwrap_or("?")[..16]);
        }
        Err(e) => println!("  Mining info   : [ERROR] {}", e),
    }
    println!();
}

fn cmd_blocks(rpc: &str, n: u64) {
    println!("╔═══════════════════════════════════════════════╗");
    println!("║         Last {} Blocks                        ║", n);
    println!("╚═══════════════════════════════════════════════╝");

    // Get current height first.
    let tip_height = match rpc_call(rpc, "getBlockHeight", serde_json::json!([])) {
        Ok(h) => h.as_u64().unwrap_or(0),
        Err(e) => { eprintln!("[ERROR] {}", e); return; }
    };

    let start = tip_height.saturating_sub(n - 1);
    println!("  {:>8}  {:>16}  {:>6}  {}", "Height", "Hash", "TXs", "Miner");
    println!("  {}", "─".repeat(70));

    for h in (start..=tip_height).rev() {
        match rpc_call(rpc, "getBlockByHeight", serde_json::json!([h])) {
            Ok(block) => {
                let hash   = &block["hash"].as_str().unwrap_or("?")[..16];
                let txs    = block["tx_count"].as_u64().unwrap_or(0);
                let miner  = &block["miner"].as_str().unwrap_or("?")[..10];
                println!("  {:>8}  {}...  {:>6}  {}...", h, hash, txs, miner);
            }
            Err(e) => println!("  {:>8}  [ERROR: {}]", h, e),
        }
    }
    println!();
}

fn cmd_block(rpc: &str, id: &str) {
    println!("╔═══════════════════════════════════════════════╗");
    println!("║         Block Details                         ║");
    println!("╚═══════════════════════════════════════════════╝");

    // Try as height first, then as hash.
    let result = if let Ok(h) = id.parse::<u64>() {
        rpc_call(rpc, "getBlockByHeight", serde_json::json!([h]))
    } else {
        rpc_call(rpc, "getBlock", serde_json::json!([id]))
    };

    match result {
        Ok(b) => print_block(&b),
        Err(e) => println!("  [ERROR] {}", e),
    }
    println!();
}

fn cmd_tx(rpc: &str, tx_id: &str) {
    println!("╔═══════════════════════════════════════════════╗");
    println!("║         Transaction Details                   ║");
    println!("╚═══════════════════════════════════════════════╝");

    match rpc_call(rpc, "getTransaction", serde_json::json!([tx_id])) {
        Ok(tx) => {
            println!("  TX ID         : {}", tx["tx_id"].as_str().unwrap_or("?"));
            println!("  From          : {}", tx["from"].as_str().unwrap_or("?"));
            println!("  To            : {}", tx["to"].as_str().unwrap_or("?"));
            println!("  Amount        : {} tokens", tx["amount"].as_str().unwrap_or("?"));
            println!("  Fee           : {} tokens", tx["fee"].as_str().unwrap_or("?"));
            println!("  Nonce         : {}", tx["nonce"]);
            println!("  Block height  : {}", tx["block_height"]);
            println!("  Block hash    : {}...", &tx["block_hash"].as_str().unwrap_or("?")[..16]);
        }
        Err(e) => println!("  [ERROR] {}", e),
    }
    println!();
}

fn cmd_address(rpc: &str, addr: &str) {
    println!("╔═══════════════════════════════════════════════╗");
    println!("║         Address Details                       ║");
    println!("╚═══════════════════════════════════════════════╝");
    println!("  Address       : {}", addr);

    match rpc_call(rpc, "getBalance", serde_json::json!([addr])) {
        Ok(bal) => println!("  Balance       : {} tokens", bal.as_str().unwrap_or("0")),
        Err(e)  => println!("  Balance       : [ERROR] {}", e),
    }

    match rpc_call(rpc, "getNonce", serde_json::json!([addr])) {
        Ok(n)  => println!("  Nonce         : {}", n),
        Err(e) => println!("  Nonce         : [ERROR] {}", e),
    }
    println!();
}

fn cmd_mining(rpc: &str) {
    println!("╔═══════════════════════════════════════════════╗");
    println!("║         Mining Information                    ║");
    println!("╚═══════════════════════════════════════════════╝");

    match rpc_call(rpc, "getMiningInfo", serde_json::json!([])) {
        Ok(m) => {
            println!("  Height        : {}", m["height"]);
            println!("  Difficulty    : {}", m["difficulty"]);
            println!("  Best hash     : {}", m["best_hash"].as_str().unwrap_or("?"));
            println!("  Target        : {}", m["target"].as_str().unwrap_or("?"));
        }
        Err(e) => println!("  [ERROR] {}", e),
    }

    match rpc_call(rpc, "getBlockTemplate", serde_json::json!([])) {
        Ok(t) => {
            println!("  Next height   : {}", t["height"]);
            println!("  Pending txs   : {}", t["tx_count"]);
        }
        Err(e) => println!("  Template      : [ERROR] {}", e),
    }
    println!();
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn print_block(b: &serde_json::Value) {
    println!("  Hash          : {}", b["hash"].as_str().unwrap_or("?"));
    println!("  Height        : {}", b["height"]);
    println!("  Parent hash   : {}...", &b["parent_hash"].as_str().unwrap_or("?")[..16]);
    println!("  State root    : {}...", &b["state_root"].as_str().unwrap_or("?")[..16]);
    println!("  Merkle root   : {}...", &b["merkle_root"].as_str().unwrap_or("?")[..16]);
    println!("  Timestamp     : {}ms", b["timestamp"]);
    println!("  Difficulty    : {}", b["difficulty"]);
    println!("  Nonce         : {}", b["nonce"]);
    println!("  Miner         : {}", b["miner"].as_str().unwrap_or("?"));
    println!("  Transactions  : {}", b["tx_count"]);
    println!("  Size          : {} bytes", b["size_bytes"]);
}

/// Send a JSON-RPC 2.0 request and return the result.
/// In production, uses actual HTTP. Here we show the request and return
/// a simulated error so the tool is runnable standalone.
fn rpc_call(
    rpc:    &str,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method":  method,
        "params":  params,
        "id":      1,
    });

    // In production: use reqwest or ureq to make the HTTP POST.
    // For now: print the request and return a "not connected" message.
    // Replace this block with an actual HTTP client for production use.
    Err(format!("not connected to {} (demo mode)", rpc))
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|w| w[0] == flag)
        .map(|w| w[1].clone())
}

fn print_help() {
    println!(r#"
Sypcoin CLI Block Explorer

USAGE:
    sypcoin-explorer <COMMAND> [OPTIONS]

COMMANDS:
    status              Network overview
    blocks [n]          Last n blocks (default: 10)
    block  <hash|height> Block details
    tx     <tx_id>      Transaction details
    address <addr>      Address balance and nonce
    mining              Mining information and next block template

OPTIONS:
    --rpc <URL>         Node RPC URL (default: http://127.0.0.1:8545)

EXAMPLES:
    sypcoin-explorer status
    sypcoin-explorer blocks 5
    sypcoin-explorer block 42
    sypcoin-explorer tx 0xabcdef...
    sypcoin-explorer address 0xYourAddress
"#);
}
