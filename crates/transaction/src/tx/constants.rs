// tx/constants.rs — Transaction-level constants.
//
// These complement the network-wide constants in primitives::constants.
// All values here are consensus-critical unless noted otherwise.

/// Transaction format version. Increment on any breaking change to to_bytes().
/// CONSENSUS CRITICAL.
pub const TX_VERSION: u8 = 1;

/// Maximum size of the optional data payload in bytes.
/// CONSENSUS CRITICAL — nodes reject transactions exceeding this.
pub const MAX_TX_DATA_SIZE: usize = 256;

/// Maximum serialized transaction size in bytes.
/// Derived: fixed fields (87 bytes) + max data (256 bytes) + 2 (data_len) = 345 bytes.
/// We set a round limit with headroom.
/// CONSENSUS CRITICAL.
pub const MAX_TX_SIZE_BYTES: usize = 156; // fixed wire format: 92 payload + 64 signature

/// Maximum age of a transaction in the mempool before eviction (5 minutes).
/// NOT consensus-critical — each node may apply its own TTL policy.
pub const TX_TTL_MS: u64 = 300_000;

/// Maximum allowed drift between a transaction timestamp and the node's
/// wall clock in milliseconds (2 minutes).
/// Soft rule — enforced at mempool admission, not at block validation.
pub const MAX_TX_FUTURE_TIME_MS: u64 = 120_000;
