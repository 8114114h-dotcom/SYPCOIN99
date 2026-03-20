// protocol/handshake.rs — P2P handshake validation.
//
// Security:
//   • chain_id must match — prevents connections to different networks.
//   • version must be compatible — prevents protocol confusion.
//   • No other messages are accepted until handshake completes.

use primitives::constants::{CHAIN_ID, PROTOCOL_VERSION};

use crate::error::NetworkError;
use crate::protocol::messages::NetworkMessage;

/// Validate an incoming Hello message.
///
/// Returns `Ok(peer_height)` if the handshake is acceptable.
pub fn validate_hello(msg: &NetworkMessage) -> Result<u64, NetworkError> {
    match msg {
        NetworkMessage::Hello { version, chain_id, height, .. } => {
            // Chain ID must match exactly.
            if *chain_id != CHAIN_ID {
                return Err(NetworkError::WrongChainId {
                    expected: CHAIN_ID,
                    got:      *chain_id,
                });
            }
            // Protocol version must be compatible (exact match for now).
            if *version != PROTOCOL_VERSION {
                return Err(NetworkError::IncompatibleVersion {
                    ours:   PROTOCOL_VERSION,
                    theirs: *version,
                });
            }
            Ok(*height)
        }
        _ => Err(NetworkError::HandshakeFailed(
            "expected Hello as first message".into()
        )),
    }
}

/// Build an acceptance HelloAck.
pub fn accept_ack() -> NetworkMessage {
    NetworkMessage::HelloAck { accepted: true, reason: None }
}

/// Build a rejection HelloAck with a reason.
pub fn reject_ack(reason: &str) -> NetworkMessage {
    NetworkMessage::HelloAck {
        accepted: false,
        reason:   Some(reason.to_owned()),
    }
}
