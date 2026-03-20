// sypcoin-cli — Command-line client for the Sypcoin node.
//
// Communicates with a running node via JSON-RPC over HTTP.
// No external crates required — uses only std + serde_json.
//
// Usage:
//   sypcoin-cli [--rpc <URL>] <COMMAND> [ARGS]
//
// Default RPC URL: http://127.0.0.1:8545

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

// ── RPC Client ────────────────────────────────────────────────────────────────

struct RpcClient {
    url: String,
}

impl RpcClient {
    fn new(url: &str) -> Self {
        RpcClient { url: url.to_string() }
    }

    /// Send a JSON-RPC request and return the response body.
    fn call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method":  method,
            "params":  params,
            "id":      1
        }).to_string();

        // Parse host and port from URL
        let addr = self.url
            .trim_start_matches("http://")
            .trim_start_matches("https://");

        let stream = TcpStream::connect(addr)
            .map_err(|e| format!("Cannot connect to node at {}: {}", self.url, e))?;

        stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
        stream.set_write_timeout(Some(Duration::from_secs(5))).ok();

        let mut stream = stream;

        // Send HTTP POST
        let request = format!(
            "POST / HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            addr, body.len(), body
        );
        stream.write_all(request.as_bytes())
            .map_err(|e| format!("Send error: {}", e))?;

        // Read response
        let mut response = String::new();
        stream.read_to_string(&mut response)
            .map_err(|e| format!("Read error: {}", e))?;

        // Extract body after \r\n\r\n
        let body_str = response
            .find("\r\n\r\n")
            .map(|p| &response[p + 4..])
            .unwrap_or(&response);

        let json: serde_json::Value = serde_json::from_str(body_str.trim())
            .map_err(|e| format!("JSON parse error: {} — response: {}", e, body_str))?;

        if let Some(err) = json.get("error") {
            return Err(format!("RPC error: {}", err));
        }

        Ok(json["result"].clone())
    }
}

// ── Commands ──────────────────────────────────────────────────────────────────

fn cmd_height(client: &RpcClient) {
    match client.call("getBlockHeight", serde_json::json!([])) {
        Ok(v)  => println!("Block height: {}", v),
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn cmd_balance(client: &RpcClient, address: &str) {
    if !address.starts_with("0x") || address.len() != 42 {
        eprintln!("Error: invalid address format. Expected 0x + 40 hex chars.");
        return;
    }
    match client.call("getBalance", serde_json::json!([address])) {
        Ok(v)  => println!("Balance: {} SYP  ({})", v.as_str().unwrap_or(&v.to_string()), address),
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn cmd_nonce(client: &RpcClient, address: &str) {
    match client.call("getNonce", serde_json::json!([address])) {
        Ok(v)  => println!("Nonce: {}", v),
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn cmd_block(client: &RpcClient, hash_or_height: &str) {
    // If it looks like a number, use getBlockByHeight; else getBlock
    let (method, params) = if hash_or_height.chars().all(|c| c.is_ascii_digit()) {
        let height: u64 = hash_or_height.parse().unwrap_or(0);
        ("getBlockByHeight", serde_json::json!([height]))
    } else {
        ("getBlock", serde_json::json!([hash_or_height]))
    };

    match client.call(method, params) {
        Ok(v)  => println!("{}", serde_json::to_string_pretty(&v).unwrap_or(v.to_string())),
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn cmd_tx(client: &RpcClient, tx_id: &str) {
    match client.call("getTransaction", serde_json::json!([tx_id])) {
        Ok(v)  => println!("{}", serde_json::to_string_pretty(&v).unwrap_or(v.to_string())),
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn cmd_send(client: &RpcClient, tx_hex: &str) {
    if !tx_hex.chars().all(|c| c.is_ascii_hexdigit()) {
        eprintln!("Error: tx_hex must be a hex-encoded signed transaction.");
        return;
    }
    match client.call("sendTransaction", serde_json::json!([tx_hex])) {
        Ok(v)  => println!("Transaction submitted. TX ID: {}", v),
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn cmd_mining(client: &RpcClient) {
    match client.call("getMiningInfo", serde_json::json!([])) {
        Ok(v)  => {
            println!("Mining Info:");
            println!("  Height:     {}", v["height"]);
            println!("  Difficulty: {}", v["difficulty"]);
            println!("  Best Hash:  {}", v["best_hash"]);
            println!("  Target:     {}", v["target"]);
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn cmd_status(client: &RpcClient) {
    println!("Connecting to node at {}...", client.url);
    let height = client.call("getBlockHeight", serde_json::json!([]));
    let mining  = client.call("getMiningInfo", serde_json::json!([]));

    match (height, mining) {
        (Ok(h), Ok(m)) => {
            println!("✅ Node is online");
            println!("   Height:     {}", h);
            println!("   Difficulty: {}", m["difficulty"]);
            println!("   Best Hash:  {}", m["best_hash"]);
        }
        (Err(e), _) | (_, Err(e)) => {
            println!("❌ Node is unreachable: {}", e);
        }
    }
}

// ── Help ──────────────────────────────────────────────────────────────────────

fn print_help() {
    println!(r#"
sypcoin-cli — Sypcoin Node Client

USAGE:
    sypcoin-cli [--rpc <URL>] <COMMAND> [ARGS]

OPTIONS:
    --rpc <URL>     RPC endpoint (default: http://127.0.0.1:8545)

COMMANDS:
    status                  Show node connection status
    height                  Get current block height
    balance <ADDRESS>       Get SYP balance for an address
    nonce   <ADDRESS>       Get transaction nonce for an address
    block   <HASH|HEIGHT>   Get block by hash or height
    tx      <TX_ID>         Get transaction by ID
    send    <TX_HEX>        Submit a signed transaction (hex-encoded)
    mining                  Get mining info (difficulty, best hash)

EXAMPLES:
    sypcoin-cli status
    sypcoin-cli height
    sypcoin-cli balance 0x6F5CaB1Aa5a40F8ae20648cF002832e5825bA6Fa
    sypcoin-cli block 100
    sypcoin-cli block 0xabcdef1234...
    sypcoin-cli tx 0xdeadbeef...
    sypcoin-cli send aabbccdd...
    sypcoin-cli mining

    # Connect to remote node:
    sypcoin-cli --rpc http://192.168.1.9:8545 status
    sypcoin-cli --rpc http://192.168.1.9:8545 balance 0x6F5CaB...
"#);
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        print_help();
        return;
    }

    // Parse --rpc flag
    let mut rpc_url = "127.0.0.1:8545".to_string();
    let mut rest: Vec<&str> = Vec::new();
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--rpc" | "-r" => {
                if i + 1 < args.len() {
                    rpc_url = args[i + 1]
                        .trim_start_matches("http://")
                        .to_string();
                    i += 2;
                } else {
                    eprintln!("Error: --rpc requires a URL");
                    std::process::exit(1);
                }
            }
            "--help" | "-h" | "help" => {
                print_help();
                return;
            }
            other => {
                rest.push(other);
                i += 1;
            }
        }
    }

    let client = RpcClient::new(&rpc_url);

    match rest.as_slice() {
        ["status"]              => cmd_status(&client),
        ["height"]              => cmd_height(&client),
        ["balance", addr]       => cmd_balance(&client, addr),
        ["nonce",   addr]       => cmd_nonce(&client, addr),
        ["block",   id]         => cmd_block(&client, id),
        ["tx",      id]         => cmd_tx(&client, id),
        ["send",    hex]        => cmd_send(&client, hex),
        ["mining"]              => cmd_mining(&client),
        []                      => print_help(),
        other => {
            eprintln!("Unknown command: {}", other.join(" "));
            eprintln!("Run 'sypcoin-cli --help' for usage.");
            std::process::exit(1);
        }
    }
}
