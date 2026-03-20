// db/schema.rs — Key encoding and column family definitions.
//
// All database keys are encoded here in one place. Centralizing key
// construction prevents subtle bugs where two subsystems use different
// encodings for the same logical key.
//
// Key formats:
//   blocks       : "b:" + hex(block_hash)         → Block bytes
//   headers      : "h:" + hex(block_hash)         → BlockHeader bytes
//   height_index : "hi:" + height_le8             → block_hash bytes (32)
//   transactions : "tx:" + hex(tx_id)             → TxRecord bytes
//   tx_by_addr   : "ta:" + hex(address) + ":" + hex(tx_id) → tx_id bytes (index)
//   snapshots    : "ss:" + height_le8             → StateSnapshot bytes
//   meta         : "meta:" + key_str              → raw bytes
//
// All multi-byte integers use little-endian encoding for consistency with
// the canonical serialization format used in the rest of the codebase.

use crypto::HashDigest;
use primitives::BlockHeight;

// ── Column / prefix constants ─────────────────────────────────────────────────

pub const PREFIX_BLOCK:        &str = "b:";
pub const PREFIX_HEADER:       &str = "h:";
pub const PREFIX_HEIGHT_INDEX: &str = "hi:";
pub const PREFIX_TX:           &str = "tx:";
pub const PREFIX_TX_BY_ADDR:   &str = "ta:";
pub const PREFIX_SNAPSHOT:     &str = "ss:";
pub const PREFIX_META:         &str = "meta:";

pub const META_TIP_HASH:       &str = "meta:tip_hash";
pub const META_CHAIN_HEIGHT:   &str = "meta:chain_height";

// ── Key constructors ──────────────────────────────────────────────────────────

pub fn block_key(hash: &HashDigest) -> String {
    format!("{}{}", PREFIX_BLOCK, hex::encode(hash.as_bytes()))
}

pub fn header_key(hash: &HashDigest) -> String {
    format!("{}{}", PREFIX_HEADER, hex::encode(hash.as_bytes()))
}

pub fn height_index_key(height: BlockHeight) -> Vec<u8> {
    let mut key = PREFIX_HEIGHT_INDEX.as_bytes().to_vec();
    key.extend_from_slice(&height.as_u64().to_le_bytes());
    key
}

pub fn tx_key(tx_id: &HashDigest) -> String {
    format!("{}{}", PREFIX_TX, hex::encode(tx_id.as_bytes()))
}

pub fn tx_addr_key(addr_hex: &str, tx_id: &HashDigest) -> String {
    format!("{}{}:{}", PREFIX_TX_BY_ADDR, addr_hex, hex::encode(tx_id.as_bytes()))
}

pub fn snapshot_key(height: BlockHeight) -> Vec<u8> {
    let mut key = PREFIX_SNAPSHOT.as_bytes().to_vec();
    key.extend_from_slice(&height.as_u64().to_le_bytes());
    key
}
