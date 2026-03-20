// p2p/discovery.rs — Peer discovery.
//
// Maintains a set of known peer addresses. On startup, seeds from DNS_SEEDS.
// Grows via GetPeers/Peers message exchange with connected peers.

use std::collections::HashSet;

use crate::protection::dns_seeds::get_seeds;

/// Manages known peer addresses.
pub struct PeerDiscovery {
    known_addrs: HashSet<String>,
    max_known:   usize,
}

impl PeerDiscovery {
    pub fn new(max_known: usize) -> Self {
        let mut disc = PeerDiscovery {
            known_addrs: HashSet::new(),
            max_known,
        };
        // Seed with DNS bootstrap addresses.
        for addr in get_seeds() {
            disc.known_addrs.insert(addr);
        }
        disc
    }

    /// Add newly discovered peer addresses.
    pub fn add_addrs(&mut self, addrs: Vec<String>) {
        for addr in addrs {
            if self.known_addrs.len() < self.max_known {
                self.known_addrs.insert(addr);
            }
        }
    }

    /// Get a list of addresses to try connecting to.
    pub fn get_candidates(&self, count: usize) -> Vec<String> {
        self.known_addrs.iter().take(count).cloned().collect()
    }

    /// Remove an address (e.g. after permanent ban).
    pub fn remove(&mut self, addr: &str) {
        self.known_addrs.remove(addr);
    }

    pub fn known_count(&self) -> usize {
        self.known_addrs.len()
    }

    /// A shareable snapshot of known addresses for GetPeers responses.
    pub fn shareable_addrs(&self, limit: usize) -> Vec<String> {
        self.known_addrs.iter().take(limit).cloned().collect()
    }
}
