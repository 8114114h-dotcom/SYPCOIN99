// constants.rs — Network-wide constants for the blockchain.
//
// Design rules:
//   • ALL consensus-critical constants are defined here and only here.
//     No magic numbers anywhere else in the codebase.
//
//   • Constants are grouped by concern and documented with their rationale.
//     Changing any value marked "CONSENSUS CRITICAL" is a hard fork.
//
//   • Monetary constants use micro-token units (u64) to avoid floating-point.
//     1 TOKEN = 1_000_000 micro-tokens (6 decimal places, like Bitcoin satoshis).

// ─── Chain identity ───────────────────────────────────────────────────────────

/// Unique identifier for this chain.
/// Used in transaction signing domain separation and p2p handshake.
/// CONSENSUS CRITICAL — changing this invalidates all existing signatures.
pub const CHAIN_ID: u64 = 1;

/// Human-readable name embedded in genesis block and p2p handshake.
pub const CHAIN_NAME: &str = "Sypcoin";
pub const COIN_NAME:   &str = "Sypcoin";
pub const COIN_SYMBOL: &str = "SYP";
pub const COIN_UNIT:   &str = "micro-SYP";

/// Protocol version negotiated during p2p handshake.
pub const PROTOCOL_VERSION: u32 = 1;

// ─── Block parameters ─────────────────────────────────────────────────────────

/// Maximum serialized block size in bytes (1 MB).
/// CONSENSUS CRITICAL — nodes reject blocks exceeding this limit.
pub const MAX_BLOCK_SIZE: u32 = 1_048_576;

/// Target time between blocks in milliseconds (10 seconds).
/// CONSENSUS CRITICAL — used by difficulty adjustment algorithm.
pub const TARGET_BLOCK_TIME_MS: u64 = 60_000;

/// Maximum number of transactions allowed in a single block.
/// CONSENSUS CRITICAL — nodes reject blocks exceeding this count.
pub const MAX_TX_PER_BLOCK: u32 = 4_096;

/// Number of blocks a coinbase (mining reward) output must wait before
/// it can be spent. Protects against chain reorganizations invalidating
/// already-spent rewards.
/// CONSENSUS CRITICAL.
pub const COINBASE_MATURITY: u64 = 100;

/// Difficulty adjustment window: recalculate every N blocks.
/// CONSENSUS CRITICAL.
pub const DIFFICULTY_ADJUSTMENT_INTERVAL: u64 = 2_016;

/// Genesis block height (always 0).
pub const GENESIS_HEIGHT: u64 = 0;

// ─── Monetary policy ──────────────────────────────────────────────────────────

/// Number of decimal places (6 → 1 TOKEN = 1_000_000 micro-tokens).
/// CONSENSUS CRITICAL — changing this breaks all amount serialization.
pub const DECIMAL_PLACES: u32 = 6;

/// Multiplier: 10^DECIMAL_PLACES = 1_000_000.
/// Used to convert whole tokens ↔ micro-tokens.
pub const MICRO_PER_TOKEN: u64 = 1_000_000;

/// Initial block reward in micro-tokens (50 tokens).
/// CONSENSUS CRITICAL.
pub const INITIAL_BLOCK_REWARD: u64 = 50 * MICRO_PER_TOKEN;

/// Reward halving interval in blocks (every 210,000 blocks ≈ 2 years).
/// CONSENSUS CRITICAL.
pub const HALVING_INTERVAL: u64 = 210_000;

/// Maximum number of halvings before reward reaches zero.
/// After 64 halvings, INITIAL_BLOCK_REWARD >> 64 = 0.
pub const MAX_HALVINGS: u32 = 64;

/// Maximum total supply in micro-tokens (21,000,000 tokens).
/// CONSENSUS CRITICAL — nodes reject blocks that would exceed this.
pub const MAX_SUPPLY_MICRO: u64 = 21_000_000 * MICRO_PER_TOKEN;

/// Minimum transaction fee in micro-tokens (0.001 tokens).
/// Nodes reject transactions below this fee from the mempool.
pub const MIN_TX_FEE_MICRO: u64 = 1_000;

/// Maximum number of transactions a single address can have in the mempool
/// simultaneously. Prevents mempool flooding from a single actor.
pub const MAX_MEMPOOL_TXS_PER_ADDRESS: usize = 64;

/// Maximum total mempool size in transactions.
pub const MAX_MEMPOOL_SIZE: usize = 10_000;

// ─── Networking ───────────────────────────────────────────────────────────────

/// Default TCP port for p2p connections.
pub const DEFAULT_P2P_PORT: u16 = 30303;

/// Default TCP port for the JSON-RPC server.
pub const DEFAULT_RPC_PORT: u16 = 8545;

/// Maximum number of outbound peer connections.
pub const MAX_OUTBOUND_PEERS: usize = 8;

/// Maximum number of inbound peer connections.
pub const MAX_INBOUND_PEERS: usize = 32;

/// Peer connection timeout in milliseconds.
pub const PEER_CONNECT_TIMEOUT_MS: u64 = 5_000;

/// Interval between peer ping messages in milliseconds.
pub const PEER_PING_INTERVAL_MS: u64 = 30_000;

/// Maximum age of a block announcement before it is ignored (milliseconds).
pub const MAX_BLOCK_PROPAGATION_AGE_MS: u64 = 60_000;

// ─── Timestamps ───────────────────────────────────────────────────────────────

/// Maximum allowed drift between a block's timestamp and the node's wall clock
/// in milliseconds (2 minutes). Blocks timestamped further in the future are
/// rejected as invalid.
/// CONSENSUS CRITICAL.
pub const MAX_FUTURE_BLOCK_TIME_MS: u64 = 120_000;

// ─── Compile-time assertions ──────────────────────────────────────────────────
// These catch accidental misconfiguration at compile time rather than runtime.

const _: () = assert!(
    MICRO_PER_TOKEN == 10u64.pow(DECIMAL_PLACES),
    "MICRO_PER_TOKEN must equal 10^DECIMAL_PLACES"
);

const _: () = assert!(
    INITIAL_BLOCK_REWARD <= MAX_SUPPLY_MICRO,
    "INITIAL_BLOCK_REWARD must not exceed MAX_SUPPLY_MICRO"
);

const _: () = assert!(
    MAX_BLOCK_SIZE >= 1024,
    "MAX_BLOCK_SIZE must be at least 1 KB"
);

const _: () = assert!(
    HALVING_INTERVAL > 0,
    "HALVING_INTERVAL must be non-zero"
);

/// Maximum chain reorganisation depth allowed (anti-51% guard).
/// Reorgs deeper than this are rejected to protect confirmed transactions.
pub const MAX_REORG_DEPTH: u64 = 100;
