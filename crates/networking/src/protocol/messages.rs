// protocol/messages.rs — All P2P network message types.
//
// Security decisions:
//   • Every message includes a MAX_SIZE check before processing.
//   • NetworkMessage is non_exhaustive so new message types can be added
//     without breaking existing match arms in downstream code.
//   • chain_id is validated in Hello before any other messages are accepted.

use serde::{Deserialize, Serialize};

use block::{Block, BlockHeader};
use crypto::HashDigest;
use transaction::Transaction;

use primitives::constants::{CHAIN_ID, PROTOCOL_VERSION};

/// Maximum serialized size of any single network message (4 MB).
pub const MAX_MESSAGE_SIZE: usize = 4 * 1024 * 1024;

/// Maximum number of headers in a single GetHeaders response.
pub const MAX_HEADERS_PER_MSG: u32 = 2_000;

/// Maximum number of blocks in a single Blocks response.
pub const MAX_BLOCKS_PER_MSG: u32 = 128;

/// All messages exchanged between peers.
#[non_exhaustive]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetworkMessage {
    // ── Handshake ─────────────────────────────────────────────────────────────
    /// Initial greeting. Must be the first message sent on a new connection.
    Hello {
        version:   u32,
        chain_id:  u64,
        height:    u64,
        best_hash: HashDigest,
        /// Peer's listen address as "ip:port" string.
        peer_addr: String,
    },
    /// Response to Hello. If `accepted=false`, the connection is closed.
    HelloAck {
        accepted: bool,
        reason:   Option<String>,
    },

    // ── Header sync ───────────────────────────────────────────────────────────
    /// Request headers starting after `from_hash`, up to `limit`.
    GetHeaders {
        from_hash: HashDigest,
        limit:     u32,
    },
    /// Response with a batch of headers.
    Headers {
        headers: Vec<BlockHeader>,
    },

    // ── Block sync ────────────────────────────────────────────────────────────
    /// Request full blocks by hash.
    GetBlocks {
        hashes: Vec<HashDigest>,
    },
    /// Response with full blocks.
    Blocks {
        blocks: Vec<Block>,
    },

    // ── Gossip ────────────────────────────────────────────────────────────────
    /// Broadcast a new transaction.
    NewTx {
        tx: Transaction,
    },
    /// Broadcast a newly mined block.
    NewBlock {
        block: Block,
    },
    /// Request a specific transaction by ID.
    GetTx {
        tx_id: HashDigest,
    },

    // ── Peer exchange ─────────────────────────────────────────────────────────
    /// Ask for known peer addresses.
    GetPeers,
    /// Response with known peer addresses.
    Peers {
        addrs: Vec<String>,
    },

    // ── Keep-alive ────────────────────────────────────────────────────────────
    Ping { nonce: u64 },
    Pong { nonce: u64 },

    // ── Disconnect ────────────────────────────────────────────────────────────
    Disconnect { reason: String },
}

impl NetworkMessage {
    /// Human-readable message type name for logging.
    pub fn type_name(&self) -> &'static str {
        match self {
            NetworkMessage::Hello { .. }      => "Hello",
            NetworkMessage::HelloAck { .. }   => "HelloAck",
            NetworkMessage::GetHeaders { .. } => "GetHeaders",
            NetworkMessage::Headers { .. }    => "Headers",
            NetworkMessage::GetBlocks { .. }  => "GetBlocks",
            NetworkMessage::Blocks { .. }     => "Blocks",
            NetworkMessage::NewTx { .. }      => "NewTx",
            NetworkMessage::NewBlock { .. }   => "NewBlock",
            NetworkMessage::GetTx { .. }      => "GetTx",
            NetworkMessage::GetPeers          => "GetPeers",
            NetworkMessage::Peers { .. }      => "Peers",
            NetworkMessage::Ping { .. }       => "Ping",
            NetworkMessage::Pong { .. }       => "Pong",
            NetworkMessage::Disconnect { .. } => "Disconnect",
        }
    }

    /// Build the Hello message for this node.
    pub fn hello(height: u64, best_hash: HashDigest, listen_addr: String) -> Self {
        NetworkMessage::Hello {
            version:   PROTOCOL_VERSION,
            chain_id:  CHAIN_ID,
            height,
            best_hash,
            peer_addr: listen_addr,
        }
    }
}
