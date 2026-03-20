# Sypcoin — Layer 1 Blockchain

A production-grade Layer 1 blockchain implemented in Rust.

---

## Architecture

```
crates/
├── 01_crypto       Ed25519 signing, SHA-256/Keccak, address derivation
├── 02_primitives   Amount, BlockHeight, Timestamp, Nonce, constants
├── 03_transaction  Transaction, builder, validator, mempool
├── 04_state        WorldState, accounts, journal, rollback, Merkle root
├── 05_block        Block, header, body, Merkle tree, PoW validation
├── 06_consensus    Mining loop, difficulty adjustment, fork choice
├── 07_execution    Block and transaction execution engine
├── 08_storage      RocksDB persistence, repositories, snapshot store
├── 09_networking   P2P, gossip, sync, rate limiting, ban list
├── 10_rpc          JSON-RPC 2.0 server
├── 11_wallet       Mnemonic, HD wallet, keystore, address book
├── 12_genesis      Genesis block and state construction
├── 13_config       Node configuration (TOML + env vars)
├── 14_event_bus    In-process event broadcasting
├── 15_metrics      Node metrics, Prometheus export, logging
├── 16_security     Anti-spam, replay protection, signature cache
├── 17_upgrade      Hard fork and soft fork schedule
└── node            Binary entry point — assembles all layers

tools/
├── miner           Standalone mining tool
├── faucet          Testnet token faucet
└── explorer        CLI block explorer

tests/
├── integration/    Full pipeline tests
├── security/       Replay, double-spend, Sybil attack tests
└── load/           Mempool and mining stress tests
```

---

## Quick Start

```bash
# Build everything
cargo build --release

# Run tests
cargo test

# Start a devnet node
cargo run --bin sypcoin -- start --network devnet

# Start with mining enabled
cargo run --bin sypcoin -- start --network devnet \
    --mine --miner 0xYourAddress

# Initialise a new data directory
cargo run --bin sypcoin -- init ./mydata mainnet

# Tools
cargo run --bin sypcoin-explorer -- status
cargo run --bin sypcoin-miner    -- --address 0xYourAddress
cargo run --bin sypcoin-faucet   -- --to 0xTestAddress --amount 10
```

---

## Configuration

Copy `crates/config/config/node.toml` and edit as needed:

```bash
cargo run --bin sypcoin -- start --config ./config/node.toml
```

Environment variable overrides:
```
SYPCOIN_LISTEN_ADDR      P2P listen address
SYPCOIN_RPC_ADDR         JSON-RPC listen address
SYPCOIN_DATA_DIR         Data directory path
SYPCOIN_LOG_LEVEL        error|warn|info|debug|trace
SYPCOIN_MINER_ADDRESS    Mining reward address
SYPCOIN_MINE_ON_START    true|false
```

---

## Genesis

Edit `crates/genesis/config/genesis.toml` to set initial accounts and
chain parameters before launching mainnet. Every node must use an
identical genesis file — any difference produces a different genesis hash
and a network split.

---

## License

MIT
