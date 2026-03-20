// p2p/peer.rs — Single peer state.

use serde::{Deserialize, Serialize};

use crypto::HashDigest;
use primitives::{BlockHeight, Timestamp};

use crate::error::PeerId;
use crate::protection::peer_score::{PeerScore, ScoreEvent};

/// Connection state of a peer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerState {
    /// TCP connected, handshake not yet complete.
    Connecting,
    /// Handshake complete, ready for messages.
    Connected,
    /// Disconnecting gracefully.
    Disconnecting,
}

/// A connected or recently-seen peer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Peer {
    pub id:        PeerId,
    pub addr:      String,          // "ip:port"
    pub state:     PeerState,
    pub version:   u32,
    pub height:    BlockHeight,
    pub best_hash: HashDigest,
    pub score:     PeerScore,
    pub last_seen: Timestamp,
    pub inbound:   bool,            // true = they connected to us
}

impl Peer {
    pub fn new(id: PeerId, addr: String, inbound: bool) -> Self {
        Peer {
            id,
            addr,
            state:     PeerState::Connecting,
            version:   0,
            height:    BlockHeight::genesis(),
            best_hash: crypto::sha256(b"genesis"),
            score:     PeerScore::new(),
            last_seen: Timestamp::now(),
            inbound,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.state == PeerState::Connected
    }

    pub fn is_banned(&self) -> bool {
        self.score.is_banned()
    }

    pub fn update_tip(&mut self, height: BlockHeight, hash: HashDigest) {
        self.height    = height;
        self.best_hash = hash;
        self.last_seen = Timestamp::now();
    }

    pub fn apply_score(&mut self, event: ScoreEvent) -> i32 {
        self.score.apply(event)
    }

    pub fn mark_connected(&mut self, version: u32) {
        self.version = version;
        self.state   = PeerState::Connected;
        self.last_seen = Timestamp::now();
    }
}
