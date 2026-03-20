// p2p/node.rs — The main network node.
//
// NetworkNode manages all peers, applies protection rules, and processes
// incoming messages into NetworkActions for the node layer to execute.
//
// Design:
//   • Pure logic — no actual TCP sockets here. The transport layer
//     (in 11_node) owns the sockets and calls handle_message().
//   • Returns Vec<NetworkAction> describing what should happen next.
//     The caller executes those actions (send messages, apply blocks, etc.)
//   • Handshake must complete before any non-Hello message is processed.

use std::collections::HashMap;

use block::Block;
use crypto::HashDigest;
use primitives::{BlockHeight, Timestamp};
use primitives::constants::{
    MAX_INBOUND_PEERS, MAX_OUTBOUND_PEERS,
};
use transaction::Transaction;

use crate::error::{NetworkError, PeerId};
use crate::gossip::broadcast::BroadcastTracker;
use crate::p2p::peer::Peer;
use crate::protection::banlist::BanList;
use crate::protection::peer_score::ScoreEvent;
use crate::protection::rate_limit::RateLimiter;
use crate::protocol::handshake::{accept_ack, reject_ack, validate_hello};
use crate::protocol::messages::NetworkMessage;

/// Actions the node layer must execute after handle_message().
#[derive(Debug)]
pub enum NetworkAction {
    /// Send a message to a specific peer.
    SendMessage { to: PeerId, msg: NetworkMessage },
    /// Broadcast a message to all connected peers.
    BroadcastMessage { msg: NetworkMessage, except: Option<PeerId> },
    /// Ban a peer and close its connection.
    BanPeer { id: PeerId, reason: String },
    /// Disconnect a peer gracefully.
    DisconnectPeer { id: PeerId },
    /// Apply a received block to the chain.
    ApplyBlock { block: Block },
    /// Add a received transaction to the mempool.
    ApplyTransaction { tx: Transaction },
    /// Initiate sync with this peer.
    RequestSync { from_peer: PeerId },
    /// No action needed.
    None,
}

/// Configuration for NetworkNode.
pub struct NetworkConfig {
    pub max_inbound:  usize,
    pub max_outbound: usize,
    pub our_height:   u64,
    pub our_best_hash: HashDigest,
    pub listen_addr:  String,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        NetworkConfig {
            max_inbound:   MAX_INBOUND_PEERS,
            max_outbound:  MAX_OUTBOUND_PEERS,
            our_height:    0,
            our_best_hash: crypto::sha256(b"genesis"),
            listen_addr:   "0.0.0.0:30303".to_owned(),
        }
    }
}

/// The network node — manages peers, protection, and message routing.
pub struct NetworkNode {
    peers:        HashMap<String, Peer>,   // hex(peer_id) → Peer
    rate_limiter: RateLimiter,
    ban_list:     BanList,
    broadcast:    BroadcastTracker,
    config:       NetworkConfig,
    our_height:   BlockHeight,
    our_best_hash: HashDigest,
}

impl NetworkNode {
    pub fn new(config: NetworkConfig) -> Self {
        let height    = config.our_height;
        let best_hash = config.our_best_hash.clone();
        NetworkNode {
            peers:        HashMap::new(),
            rate_limiter: RateLimiter::with_defaults(),
            ban_list:     BanList::new(),
            broadcast:    BroadcastTracker::new(10_000),
            config,
            our_height:    BlockHeight::new(height),
            our_best_hash: best_hash,
        }
    }

    // ── Peer management ───────────────────────────────────────────────────────

    /// Register a new peer connection. Returns the Hello message to send.
    pub fn add_peer(
        &mut self,
        peer_id: PeerId,
        addr:    String,
        inbound: bool,
    ) -> Result<NetworkMessage, NetworkError> {
        let key = hex::encode(&peer_id);

        if self.ban_list.is_banned(&peer_id) {
            return Err(NetworkError::PeerBanned);
        }
        if self.peers.contains_key(&key) {
            return Err(NetworkError::PeerAlreadyConnected);
        }

        let inbound_count  = self.peers.values().filter(|p| p.inbound).count();
        let outbound_count = self.peers.values().filter(|p| !p.inbound).count();

        if inbound && inbound_count >= self.config.max_inbound {
            return Err(NetworkError::MaxPeersReached);
        }
        if !inbound && outbound_count >= self.config.max_outbound {
            return Err(NetworkError::MaxPeersReached);
        }

        self.peers.insert(key, Peer::new(peer_id, addr, inbound));

        Ok(NetworkMessage::hello(
            self.our_height.as_u64(),
            self.our_best_hash.clone(),
            self.config.listen_addr.clone(),
        ))
    }

    /// Remove a peer (disconnect or ban).
    pub fn remove_peer(&mut self, peer_id: &PeerId) {
        let key = hex::encode(peer_id);
        self.peers.remove(&key);
        self.rate_limiter.remove(peer_id);
    }

    // ── Message handling ──────────────────────────────────────────────────────

    /// Process an incoming message from a peer.
    ///
    /// Returns a list of actions the node layer must execute.
    pub fn handle_message(
        &mut self,
        from:    PeerId,
        msg:     NetworkMessage,
        now:     Timestamp,
    ) -> Vec<NetworkAction> {
        let key = hex::encode(&from);

        // 1. Rate limit check.
        // Backpressure: rate limiter enforces max 100 msg/sec per peer.
        // Exceeding peers are penalised — accumulate enough and they get banned.
        if !self.rate_limiter.check(&from, now) {
            let _ = self.penalize(&from, ScoreEvent::SpamDetected);
            return vec![NetworkAction::BanPeer {
                id:     from,
                reason: "rate limit exceeded".into(),
            }];
        }

        // 2. Peer must exist.
        let peer_exists = self.peers.contains_key(&key);
        if !peer_exists {
            return vec![NetworkAction::None];
        }

        // 3. Dispatch by message type.
        match msg {
            NetworkMessage::Hello { .. } => self.handle_hello(from, msg),

            NetworkMessage::HelloAck { accepted, .. } => {
                if !accepted {
                    vec![NetworkAction::DisconnectPeer { id: from }]
                } else {
                    vec![NetworkAction::None]
                }
            }

            // Must be connected (handshake done) for all other messages.
            other => {
                let connected = self.peers.get(&key)
                    .map(|p| p.is_connected())
                    .unwrap_or(false);

                if !connected {
                    vec![NetworkAction::DisconnectPeer { id: from }]
                } else {
                    self.handle_post_handshake(from, other)
                }
            }
        }
    }

    // ── Internal handlers ─────────────────────────────────────────────────────

    fn handle_hello(&mut self, from: PeerId, msg: NetworkMessage) -> Vec<NetworkAction> {
        match validate_hello(&msg) {
            Err(e) => {
                let ack = reject_ack(&e.to_string());
                vec![
                    NetworkAction::SendMessage { to: from, msg: ack },
                    NetworkAction::DisconnectPeer { id: from },
                ]
            }
            Ok(peer_height) => {
                let key = hex::encode(&from);
                if let Some(peer) = self.peers.get_mut(&key) {
                    peer.mark_connected(primitives::constants::PROTOCOL_VERSION);
                    peer.height = BlockHeight::new(peer_height);
                }

                let mut actions = vec![
                    NetworkAction::SendMessage { to: from, msg: accept_ack() },
                ];

                // If the peer is ahead of us, start sync.
                if peer_height > self.our_height.as_u64() {
                    actions.push(NetworkAction::RequestSync { from_peer: from });
                }

                actions
            }
        }
    }

    fn handle_post_handshake(
        &mut self,
        from: PeerId,
        msg:  NetworkMessage,
    ) -> Vec<NetworkAction> {
        match msg {
            NetworkMessage::Ping { nonce } =>
                vec![NetworkAction::SendMessage {
                    to:  from,
                    msg: NetworkMessage::Pong { nonce },
                }],

            NetworkMessage::Pong { .. } => {
                // Update last_seen.
                let key = hex::encode(&from);
                if let Some(peer) = self.peers.get_mut(&key) {
                    peer.last_seen = Timestamp::now();
                }
                vec![NetworkAction::None]
            }

            NetworkMessage::NewBlock { block } => {
                let hash = block.hash();
                if self.broadcast.is_new_block(&hash) {
                    self.penalize(&from, ScoreEvent::ValidBlock);
                    vec![
                        NetworkAction::ApplyBlock { block: block.clone() },
                        NetworkAction::BroadcastMessage {
                            msg:    NetworkMessage::NewBlock { block },
                            except: Some(from),
                        },
                    ]
                } else {
                    vec![NetworkAction::None] // already seen
                }
            }

            NetworkMessage::NewTx { tx } => {
                let tx_id = tx.tx_id().clone();
                if self.broadcast.is_new_tx(&tx_id) {
                    self.penalize(&from, ScoreEvent::ValidTx);
                    vec![
                        NetworkAction::ApplyTransaction { tx: tx.clone() },
                        NetworkAction::BroadcastMessage {
                            msg:    NetworkMessage::NewTx { tx },
                            except: Some(from),
                        },
                    ]
                } else {
                    vec![NetworkAction::None]
                }
            }

            NetworkMessage::GetPeers => {
                let addrs = self.peers.values()
                    .filter(|p| p.is_connected())
                    .map(|p| p.addr.clone())
                    .take(30)
                    .collect();
                vec![NetworkAction::SendMessage {
                    to:  from,
                    msg: NetworkMessage::Peers { addrs },
                }]
            }

            NetworkMessage::Disconnect { .. } => {
                vec![NetworkAction::DisconnectPeer { id: from }]
            }

            // Headers, Blocks, etc. are handled by the sync manager in node layer.
            other => vec![NetworkAction::SendMessage {
                to:  from,
                msg: other, // pass-through for sync manager
            }],
        }
    }

    // ── Gossip ────────────────────────────────────────────────────────────────

    /// Broadcast a new transaction to all connected peers.
    pub fn broadcast_tx(&mut self, tx: Transaction) -> usize {
        let tx_id = tx.tx_id().clone();
        if !self.broadcast.is_new_tx(&tx_id) {
            return 0;
        }
        self.connected_count()
    }

    /// Broadcast a new block to all connected peers.
    pub fn broadcast_block(&mut self, block: &Block) -> usize {
        if !self.broadcast.is_new_block(&block.hash()) {
            return 0;
        }
        self.connected_count()
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    pub fn connected_count(&self) -> usize {
        self.peers.values().filter(|p| p.is_connected()).count()
    }

    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    pub fn best_peer(&self) -> Option<&Peer> {
        self.peers.values()
            .filter(|p| p.is_connected())
            .max_by_key(|p| p.height.as_u64())
    }

    /// Update our own chain tip (called after mining or applying a block).
    pub fn update_our_tip(&mut self, height: BlockHeight, hash: HashDigest) {
        self.our_height    = height;
        self.our_best_hash = hash;
    }

    // ── Internal ──────────────────────────────────────────────────────────────

    fn penalize(&mut self, peer_id: &PeerId, event: ScoreEvent) -> bool {
        let key = hex::encode(peer_id);
        if let Some(peer) = self.peers.get_mut(&key) {
            let _score = peer.apply_score(event);
            if peer.is_banned() {
                self.ban_list.ban(peer_id, "score below ban threshold");
                return true; // should ban
            }
        }
        false
    }
}
