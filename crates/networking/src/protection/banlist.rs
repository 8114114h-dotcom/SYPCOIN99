// protection/banlist.rs — Peer ban list.
//
// Banned peers are refused connections immediately.
// Bans can be time-limited or permanent (until node restart).

use std::collections::HashMap;

use primitives::Timestamp;

use crate::error::PeerId;

/// A ban record for a single peer.
#[derive(Clone, Debug)]
pub struct BanEntry {
    pub reason:    String,
    pub banned_at: Timestamp,
    /// None = permanent ban (for this session).
    pub expires_at: Option<Timestamp>,
}

/// List of banned peer IDs.
pub struct BanList {
    entries: HashMap<String, BanEntry>,
}

impl BanList {
    pub fn new() -> Self {
        BanList { entries: HashMap::new() }
    }

    /// Ban a peer permanently (for this session).
    pub fn ban(&mut self, peer_id: &PeerId, reason: &str) {
        self.entries.insert(hex::encode(peer_id), BanEntry {
            reason:     reason.to_owned(),
            banned_at:  Timestamp::now(),
            expires_at: None,
        });
    }

    /// Ban a peer for a limited duration.
    pub fn ban_for(&mut self, peer_id: &PeerId, reason: &str, duration_ms: u64) {
        let now = Timestamp::now();
        let expires = now.checked_add_millis(duration_ms).unwrap_or(now);
        self.entries.insert(hex::encode(peer_id), BanEntry {
            reason:     reason.to_owned(),
            banned_at:  now,
            expires_at: Some(expires),
        });
    }

    /// Returns `true` if the peer is currently banned.
    pub fn is_banned(&self, peer_id: &PeerId) -> bool {
        match self.entries.get(&hex::encode(peer_id)) {
            None        => false,
            Some(entry) => match entry.expires_at {
                None         => true,
                Some(expiry) => Timestamp::now().is_before(&expiry),
            },
        }
    }

    /// Unban a peer explicitly.
    pub fn unban(&mut self, peer_id: &PeerId) {
        self.entries.remove(&hex::encode(peer_id));
    }

    /// Remove all expired bans.
    pub fn cleanup_expired(&mut self) {
        let now = Timestamp::now();
        self.entries.retain(|_, entry| {
            match entry.expires_at {
                None         => true,  // permanent, keep
                Some(expiry) => now.is_before(&expiry),
            }
        });
    }

    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
}

impl Default for BanList {
    fn default() -> Self { Self::new() }
}
