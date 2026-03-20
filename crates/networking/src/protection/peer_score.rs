// protection/peer_score.rs — Peer reputation scoring.
//
// Peers accumulate positive/negative scores based on their behaviour.
// A peer whose score drops below BAN_THRESHOLD is permanently banned.
//
// Design (similar to Bitcoin Core's misbehaviour scoring):
//   • Good behaviour earns points (valid blocks, valid txs).
//   • Bad behaviour loses points (invalid blocks, spam, timeouts).
//   • Ban is permanent for the session; persistent bans stored in BanList.

use serde::{Deserialize, Serialize};

/// Score below which a peer is banned.
pub const BAN_THRESHOLD: i32 = -100;

/// Score events and their point values.
#[derive(Clone, Copy, Debug)]
pub enum ScoreEvent {
    ValidBlock,       // +20
    InvalidBlock,     // -50
    ValidTx,          // +5
    InvalidTx,        // -20
    InvalidMessage,   // -10
    Timeout,          // -5
    SpamDetected,     // -30
    HelpfulSync,      // +10
}

impl ScoreEvent {
    pub fn points(self) -> i32 {
        match self {
            ScoreEvent::ValidBlock     =>  20,
            ScoreEvent::InvalidBlock   => -50,
            ScoreEvent::ValidTx        =>   5,
            ScoreEvent::InvalidTx      => -20,
            ScoreEvent::InvalidMessage => -10,
            ScoreEvent::Timeout        =>  -5,
            ScoreEvent::SpamDetected   => -30,
            ScoreEvent::HelpfulSync    =>  10,
        }
    }
}

/// Per-peer reputation score.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerScore {
    score: i32,
}

impl PeerScore {
    pub fn new() -> Self {
        PeerScore { score: 0 }
    }

    /// Apply a score event and return the new score.
    pub fn apply(&mut self, event: ScoreEvent) -> i32 {
        // Clamp to [-200, 200] to prevent runaway values.
        self.score = (self.score + event.points()).clamp(-200, 200);
        self.score
    }

    pub fn value(&self) -> i32 { self.score }

    /// Returns `true` if this peer should be banned.
    pub fn is_banned(&self) -> bool {
        self.score <= BAN_THRESHOLD
    }
}

impl Default for PeerScore {
    fn default() -> Self { Self::new() }
}
