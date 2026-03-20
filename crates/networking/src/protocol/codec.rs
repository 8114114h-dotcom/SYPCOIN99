// protocol/codec.rs — Message encoding and decoding.
//
// Wire format: length(4 bytes LE) || bincode(NetworkMessage)
//
// The 4-byte length prefix allows framing on a stream socket:
// the receiver reads 4 bytes first, then reads exactly that many bytes.
//
// MAX_MESSAGE_SIZE is enforced on both encode and decode to prevent
// memory exhaustion attacks via crafted length prefixes.

use crate::error::NetworkError;
use crate::protocol::messages::{NetworkMessage, MAX_MESSAGE_SIZE};

/// Encode a message to wire bytes: length_le4 || bincode_payload.
pub fn encode(msg: &NetworkMessage) -> Result<Vec<u8>, NetworkError> {
    let payload = bincode::serialize(msg)
        .map_err(|e| NetworkError::CodecError(e.to_string()))?;

    if payload.len() > MAX_MESSAGE_SIZE {
        return Err(NetworkError::MessageTooLarge {
            max: MAX_MESSAGE_SIZE,
            got: payload.len(),
        });
    }

    let mut wire = Vec::with_capacity(4 + payload.len());
    wire.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    wire.extend_from_slice(&payload);
    Ok(wire)
}

/// Decode a message from wire bytes (length prefix already stripped).
///
/// Caller is responsible for reading exactly `length` bytes from the stream.
pub fn decode(payload: &[u8]) -> Result<NetworkMessage, NetworkError> {
    if payload.len() > MAX_MESSAGE_SIZE {
        return Err(NetworkError::MessageTooLarge {
            max: MAX_MESSAGE_SIZE,
            got: payload.len(),
        });
    }

    bincode::deserialize(payload)
        .map_err(|e| NetworkError::CodecError(e.to_string()))
}

/// Read the 4-byte length prefix from a wire buffer.
///
/// Returns `None` if the buffer has fewer than 4 bytes.
pub fn read_length(buf: &[u8]) -> Option<usize> {
    if buf.len() < 4 {
        return None;
    }
    let mut arr = [0u8; 4];
    arr.copy_from_slice(&buf[..4]);
    Some(u32::from_le_bytes(arr) as usize)
}
