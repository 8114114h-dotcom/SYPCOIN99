use block::BlockBuilder;
use config::NodeConfig;
use consensus::Miner;
use event_bus::ChainEvent;
use primitives::Timestamp;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use crate::node::lifecycle::{build_services, shutdown_services};
use crate::node::services::NodeServices;
use crate::node::shutdown::ShutdownHandle;

struct RateLimiter {
    counts: HashMap<String, (u64, std::time::Instant)>,
    max_per_minute: u64,
}

impl RateLimiter {
    fn new(max_per_minute: u64) -> Self {
        RateLimiter { counts: HashMap::new(), max_per_minute }
    }
    fn allow(&mut self, ip: &str) -> bool {
        let now = std::time::Instant::now();
        let entry = self.counts.entry(ip.to_string()).or_insert((0, now));
        if now.duration_since(entry.1).as_secs() >= 60 {
            *entry = (1, now); true
        } else if entry.0 < self.max_per_minute {
            entry.0 += 1; true
        } else { false }
    }
}

pub struct NodeRunner {
    pub config:    NodeConfig,
    pub services:  NodeServices,
    shutdown:      ShutdownHandle,
    mine_on_start: bool,
    miner_address: Option<String>,
    balances:      Arc<Mutex<HashMap<String, String>>>,
}

impl NodeRunner {
    pub fn new(
        config:        NodeConfig,
        mine_on_start: bool,
        miner_address: Option<String>,
    ) -> Result<Self, String> {
        let services = build_services(&config)?;
        let shutdown = ShutdownHandle::new();
        let balances = Arc::new(Mutex::new(HashMap::new()));
        Ok(NodeRunner { config, services, shutdown, mine_on_start, miner_address, balances })
    }

    pub fn start(&mut self) {
        let tip    = self.services.blockchain.tip();
        let height = tip.height();
        let hash   = tip.hash();
        self.services.event_bus.publish(ChainEvent::NodeStarted { height, tip_hash: hash });
        metrics::init_tracing(&self.config.log_level);
        eprintln!("[INFO] Sypcoin node started — height={} difficulty={}",
            height, self.services.blockchain.current_difficulty());
        if self.mine_on_start { self.start_mining_round(); }
    }

    pub fn run(&mut self) {
        let rx               = self.services.event_bus.subscribe();
        let rpc_addr         = self.config.rpc.listen_addr.clone();
        let shutdown_rpc     = self.shutdown.clone();
        let chain_height     = Arc::new(Mutex::new(self.services.blockchain.height().as_u64()));
        let chain_height_rpc = chain_height.clone();
        let balances_rpc     = self.balances.clone();
        let pending_txs: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let pending_txs_rpc  = pending_txs.clone();

        std::thread::spawn(move || {
            let listener = match TcpListener::bind(&rpc_addr) {
                Ok(l)  => { eprintln!("[INFO] RPC listening on {}", rpc_addr); l }
                Err(e) => { eprintln!("[ERROR] RPC bind failed: {}", e); return; }
            };
            listener.set_nonblocking(true).ok();
            let rate_limiter = Arc::new(Mutex::new(RateLimiter::new(60)));

            while !shutdown_rpc.is_triggered() {
                match listener.accept() {
                    Ok((mut stream, peer_addr)) => {
                        // Recover from poisoned locks instead of panicking.
                        let height = chain_height_rpc.lock()
                            .unwrap_or_else(|p| p.into_inner())
                            .clone();
                        let balances = balances_rpc.lock()
                            .unwrap_or_else(|p| p.into_inner())
                            .clone();
                        let pending_ref = pending_txs_rpc.clone();
                        let rl          = rate_limiter.clone();
                        let ip          = peer_addr.ip().to_string();

                        std::thread::spawn(move || {
                            if !rl.lock().unwrap_or_else(|p| p.into_inner()).allow(&ip) {
                                stream.write_all(b"HTTP/1.1 429 Too Many Requests\r\nContent-Length: 0\r\n\r\n").ok();
                                return;
                            }
                            let mut buf = vec![0u8; 65536];
                            let n = match stream.read(&mut buf) {
                                Ok(n) if n > 0 => n, _ => return,
                            };
                            let raw = String::from_utf8_lossy(&buf[..n]);
                            if raw.starts_with("OPTIONS") {
                                stream.write_all(b"HTTP/1.1 200 OK\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: POST\r\nAccess-Control-Allow-Headers: Content-Type\r\n\r\n").ok();
                                return;
                            }
                            let body = raw.find("\r\n\r\n").map(|p| raw[p+4..].trim().to_string()).unwrap_or_default();
                            if body.is_empty() || body.len() > 32768 { return; }

                            let rb = Self::handle_rpc(&body, height, &balances, &pending_ref);
                            let http = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\n\r\n{}", rb.len(), rb);
                            stream.write_all(http.as_bytes()).ok();
                        });
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
        });

        let need_mine        = self.mine_on_start;
        let mut last_mine_ms = 0u64;

        while !self.shutdown.is_triggered() {
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

            if let Ok(mut h) = chain_height.lock() { *h = self.services.blockchain.height().as_u64(); }

            if let Some(addr_str) = &self.miner_address {
                if let Ok(addr) = crypto::Address::from_checksum_hex(addr_str) {
                    let bal   = self.services.executor.state().get_balance(&addr);
                    let micro = bal.as_micro();
                    let disp  = format!("{}.{:06}", micro / 1_000_000, micro % 1_000_000);
                    if let Ok(mut b) = self.balances.lock() { b.insert(addr_str.clone(), disp); }
                }
            }

            let txs: Vec<String> = pending_txs.lock()
                .unwrap_or_else(|p| p.into_inner())
                .drain(..)
                .collect();
            for tx_hex in txs {
                if let Ok(bytes) = hex::decode(&tx_hex) {
                    if let Ok(tx) = transaction::Transaction::from_wallet_bytes(&bytes).map_err(|_| bincode::Error::from(bincode::ErrorKind::Custom("wallet parse failed".into()))) {
                        // 1. التحقق من التوقيع والبنية
                        if let Err(e) = transaction::TransactionValidator::validate_structure(&tx) {
                            eprintln!("[WARN] TX invalid structure: {}", e);
                            continue;
                        }
                        // 2. التحقق من الرصيد والـ nonce
                        let sender_balance = self.services.executor.state().get_balance(tx.from());
                        let sender_nonce   = self.services.executor.state().get_nonce(tx.from());
                        let now            = primitives::Timestamp::now();
                        if let Err(e) = transaction::TransactionValidator::validate_against_state(
                            &tx, sender_balance, sender_nonce, now
                        ) {
                            eprintln!("[WARN] TX state invalid: {}", e);
                            continue;
                        }
                        // 3. أضفها للـ mempool
                        match self.services.mempool.add(tx) {
                            Ok(_)  => eprintln!("[INFO] TX added to mempool"),
                            Err(e) => eprintln!("[WARN] TX rejected: {}", e),
                        }
                    } else {
                        eprintln!("[WARN] TX deserialization failed");
                    }
                }
            }

            for event in rx.drain() { self.handle_event(event); }

            // Mine every 1 second minimum
            if need_mine && (now_ms - last_mine_ms) >= 60000 {
                self.start_mining_round();
                last_mine_ms = now_ms;
            }

            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        self.stop();
    }

    fn handle_rpc(
        body:        &str,
        height:      u64,
        balances:    &HashMap<String, String>,
        pending_txs: &Arc<Mutex<Vec<String>>>,
    ) -> String {
        if body.contains("getBlockHeight") {
            format!("{{\"jsonrpc\":\"2.0\",\"result\":{},\"id\":1}}", height)
        } else if body.contains("getBalance") {
            let addr = body.split('"').find(|s| s.starts_with("0x") && s.len() == 42).unwrap_or("").to_string();
            let bal  = balances.get(&addr).cloned().unwrap_or("0.000000".to_string());
            format!("{{\"jsonrpc\":\"2.0\",\"result\":\"{}\",\"id\":1}}", bal)
        } else if body.contains("sendTransaction") {
            let tx_hex = body.split('"')
                .find(|s| s.len() > 20 && s.chars().all(|c| c.is_ascii_hexdigit()))
                .unwrap_or("").to_string();
            if tx_hex.is_empty() {
                "{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32602,\"message\":\"missing tx_hex\"},\"id\":1}".to_string()
            } else {
                match hex::decode(&tx_hex) {
                    Ok(bytes) if bytes.len() > 64 => {
                        let tx_id = format!("{:016x}{:016x}", bytes.len() as u64, bytes[0] as u64);
                        if let Ok(mut q) = pending_txs.lock() { q.push(tx_hex); }
                        format!("{{\"jsonrpc\":\"2.0\",\"result\":\"{}\",\"id\":1}}", tx_id)
                    }
                    _ => "{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32602,\"message\":\"invalid tx\"},\"id\":1}".to_string()
                }
            }
        } else if body.contains("getNonce") {
            "{\"jsonrpc\":\"2.0\",\"result\":0,\"id\":1}".to_string()
        } else if body.contains("getMiningInfo") {
            format!("{{\"jsonrpc\":\"2.0\",\"result\":{{\"height\":{},\"difficulty\":1,\"best_hash\":\"0000\",\"target\":\"ffff\"}},\"id\":1}}", height)
        } else {
            "{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32601,\"message\":\"Method not found\"},\"id\":1}".to_string()
        }
    }

    pub fn trigger_shutdown(&self) { self.shutdown.trigger(); }

    fn handle_event(&mut self, event: ChainEvent) {
        match &event {
            ChainEvent::BlockAdded { height, tx_count, .. } => {
                self.services.metrics.update_chain(
                    height.as_u64(),
                    hex::encode(self.services.blockchain.tip().hash().as_bytes()),
                    *tx_count as u64,
                );
            }
            ChainEvent::NewTransaction { .. } => {
                self.services.metrics.update_mempool(self.services.mempool.len());
            }
            ChainEvent::PeerConnected { .. } | ChainEvent::PeerDisconnected { .. } => {
                self.services.metrics.update_network(self.services.network.connected_count(), 0, 0, 0, 0);
            }
            ChainEvent::BlockMined { nonce, .. } => {
                self.services.metrics.update_mining(*nonce, self.services.metrics.last_block_time_ms);
            }
            ChainEvent::NodeStopping => { eprintln!("[INFO] Shutdown event received."); }
            _ => {}
        }
    }

    fn start_mining_round(&mut self) {
        let miner_addr = match &self.miner_address {
            Some(addr) => match crypto::Address::from_checksum_hex(addr) {
                Ok(a) => a, Err(_) => { eprintln!("[WARN] Invalid miner address"); return; }
            },
            None => return,
        };

        let tip        = self.services.blockchain.tip();
        let height     = tip.height().next();
        let parent     = tip.hash();
        let difficulty = self.services.blockchain.current_difficulty();
        let state_root = self.services.executor.state().state_root().clone();

        // Arc<Transaction> — cheap clone (atomic counter only)
        let txs: Vec<_> = self.services.mempool
            .top_n(primitives::constants::MAX_TX_PER_BLOCK as usize)
            .into_iter()
            .map(|arc_tx| (*arc_tx).clone())
            .collect();

        let template = match BlockBuilder::new()
            .height(height).parent_hash(parent).state_root(state_root)
            .miner(miner_addr.clone()).difficulty(difficulty)
            .timestamp(Timestamp::now()).transactions(txs).build()
        {
            Ok(t) => t, Err(e) => { eprintln!("[WARN] Block template failed: {}", e); return; }
        };

        let shutdown = self.shutdown.clone();
        let miner    = Miner::new(miner_addr);

        match miner.mine(template, move || shutdown.is_triggered()) {
            Ok(result) => {
                let block = result.block;
                let hash  = block.hash();
                let h     = block.height();
                let nonce = result.nonce_found;
                let ms    = result.elapsed_ms;

                eprintln!("[INFO] Block mined! height={} nonce={} time={}ms", h, nonce, ms);

                if let Err(e) = self.services.storage.save_block(&block) {
                    eprintln!("[WARN] Failed to save block: {}", e);
                }
                if let Err(e) = self.services.blockchain.add_block(block.clone()) {
                    eprintln!("[WARN] Block rejected: {}", e); return;
                }
                if let Err(e) = self.services.executor.execute_block(&block) {
                    eprintln!("[WARN] Execution failed: {}", e); return;
                }
                // Save state snapshot after each block for persistence.
                let snap = self.services.executor.snapshot();
                let _ = self.services.storage.save_snapshot(&snap);

                for tx in block.transactions() {
                    self.services.mempool.remove(tx.tx_id());
                }

                self.services.event_bus.publish(ChainEvent::BlockMined {
                    block_hash: hash.clone(), height: h, nonce, elapsed_ms: ms,
                });
                self.services.event_bus.publish(ChainEvent::BlockAdded {
                    block_hash: hash, height: h, tx_count: block.tx_count(), timestamp: Timestamp::now(),
                });
            }
            Err(_) => {}
        }
    }

    fn stop(&mut self) {
        eprintln!("[INFO] Stopping node...");
        self.services.event_bus.publish(ChainEvent::NodeStopping);
        shutdown_services(&mut self.services);
        eprintln!("[INFO] Node stopped.");
    }
}
