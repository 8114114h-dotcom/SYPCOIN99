// codec.rs — Serialization helpers for storage.
//
// bincode is used for all on-disk serialization. It produces compact binary
// output and is significantly faster than JSON or CBOR for struct-heavy data.
//
// HashDigest reconstruction:
//   The crypto crate exposes HashDigest(pub(crate) [u8; 32]).
//   We reconstruct it via sha256() of the stored bytes — but that changes the
//   value. Instead we use a zero-copy wrapper approach: we store raw 32 bytes
//   and reconstruct via crypto::sha256 of a sentinel + the bytes.
//
//   Actually the cleanest approach: we store HashDigest values as their
//   as_bytes() output (32 raw bytes) and reconstruct via a local wrapper.
//   Since HashDigest is just a [u8;32] newtype, bincode serializes it
//   correctly via serde derive — no manual reconstruction needed.

use crate::error::StorageError;

/// Serialize a value to bytes using bincode.
pub fn serialize<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, StorageError> {
    bincode::serialize(value)
        .map_err(|e| StorageError::SerializationError(e.to_string()))
}

/// Deserialize a value from bytes using bincode.
pub fn deserialize<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, StorageError> {
    bincode::deserialize(bytes)
        .map_err(|e| StorageError::DeserializationError(e.to_string()))
}

/// Reconstruct a HashDigest from 32 raw bytes.
///
/// HashDigest is serialized by bincode as its inner [u8; 32] via serde.
/// This function provides an explicit reconstruction path for cases where
/// we store raw hash bytes (e.g. height_index values).
pub fn hash_digest_from_bytes(bytes: &[u8]) -> Result<crypto::HashDigest, StorageError> {
    if bytes.len() != 32 {
        return Err(StorageError::CorruptedData(
            format!("expected 32-byte hash, got {} bytes", bytes.len())
        ));
    }
    // We reconstruct by serializing a dummy and replacing — but the cleanest
    // way is to use bincode round-trip on the 32-byte slice.
    // bincode encodes [u8;32] as 32 raw bytes with no length prefix.
    bincode::deserialize::<crypto::HashDigest>(bytes)
        .map_err(|e| StorageError::DeserializationError(e.to_string()))
}
