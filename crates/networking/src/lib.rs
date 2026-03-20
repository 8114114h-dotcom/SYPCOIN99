#![allow(dead_code, unused_imports)]
#![allow(dead_code, unused_imports)]
// lib.rs — Public API for the networking crate.
//
//   use networking::{NetworkNode, NetworkConfig, NetworkAction, NetworkMessage};
//   use networking::{PeerId, NetworkError};

mod error;

mod protocol {
    pub(crate) mod codec;
    pub(crate) mod handshake;
    pub(crate) mod messages;
}

mod p2p {
    pub(crate) mod discovery;
    pub(crate) mod node;
    pub(crate) mod peer;
}

mod sync {
    pub(crate) mod block_sync;
    pub(crate) mod fast_sync;
    pub(crate) mod header_sync;
}

mod gossip {
    pub(crate) mod broadcast;
    pub(crate) mod router;
}

mod protection {
    pub(crate) mod banlist;
    pub(crate) mod dns_seeds;
    pub(crate) mod peer_score;
    pub(crate) mod rate_limit;
}

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use error::{NetworkError, PeerId};
pub use p2p::node::{NetworkAction, NetworkConfig, NetworkNode};
pub use p2p::peer::{Peer, PeerState};
pub use p2p::discovery::PeerDiscovery;
pub use protocol::messages::NetworkMessage;
pub use protocol::codec::{encode, decode, read_length};
pub use protocol::handshake::{validate_hello, accept_ack, reject_ack};
pub use protection::peer_score::{PeerScore, ScoreEvent};
pub use protection::banlist::BanList;
pub use protection::rate_limit::RateLimiter;
pub use protection::dns_seeds::get_seeds;
pub use sync::header_sync::HeaderSync;
pub use sync::block_sync::BlockSync;
pub use sync::fast_sync::{FastSync, FastSyncState};
pub use gossip::broadcast::BroadcastTracker;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crypto::{KeyPair, sha256};
    use primitives::{BlockHeight, Timestamp};
    use primitives::constants::{CHAIN_ID, PROTOCOL_VERSION};

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn make_peer_id(seed: u8) -> PeerId {
        let mut id = [0u8; 32];
        id[0] = seed;
        id
    }

    fn make_node() -> NetworkNode {
        NetworkNode::new(NetworkConfig::default())
    }

    fn valid_hello(height: u64) -> NetworkMessage {
        NetworkMessage::Hello {
            version:   PROTOCOL_VERSION,
            chain_id:  CHAIN_ID,
            height,
            best_hash: sha256(b"best"),
            peer_addr: "127.0.0.1:30303".into(),
        }
    }

    // ── Codec ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_encode_decode_roundtrip() {
        let msg = NetworkMessage::Ping { nonce: 42 };
        let wire = encode(&msg).unwrap();
        // Strip 4-byte length prefix.
        let payload = &wire[4..];
        let decoded = decode(payload).unwrap();
        assert!(matches!(decoded, NetworkMessage::Ping { nonce: 42 }));
    }

    #[test]
    fn test_read_length() {
        let msg  = NetworkMessage::Pong { nonce: 1 };
        let wire = encode(&msg).unwrap();
        let len  = read_length(&wire).unwrap();
        assert_eq!(len, wire.len() - 4);
    }

    // ── Handshake ─────────────────────────────────────────────────────────────

    #[test]
    fn test_valid_hello_accepted() {
        let msg    = valid_hello(10);
        let height = validate_hello(&msg).unwrap();
        assert_eq!(height, 10);
    }

    #[test]
    fn test_wrong_chain_id_rejected() {
        let msg = NetworkMessage::Hello {
            version:   PROTOCOL_VERSION,
            chain_id:  999, // wrong
            height:    0,
            best_hash: sha256(b"x"),
            peer_addr: "".into(),
        };
        assert!(matches!(
            validate_hello(&msg),
            Err(NetworkError::WrongChainId { .. })
        ));
    }

    #[test]
    fn test_wrong_version_rejected() {
        let msg = NetworkMessage::Hello {
            version:   999, // wrong
            chain_id:  CHAIN_ID,
            height:    0,
            best_hash: sha256(b"x"),
            peer_addr: "".into(),
        };
        assert!(matches!(
            validate_hello(&msg),
            Err(NetworkError::IncompatibleVersion { .. })
        ));
    }

    #[test]
    fn test_non_hello_rejected_in_handshake() {
        let msg = NetworkMessage::Ping { nonce: 1 };
        assert!(validate_hello(&msg).is_err());
    }

    // ── NetworkNode ───────────────────────────────────────────────────────────

    #[test]
    fn test_add_peer_success() {
        let mut node = make_node();
        let peer_id  = make_peer_id(1);
        let hello    = node.add_peer(peer_id, "127.0.0.1:30303".into(), true);
        assert!(hello.is_ok());
        assert_eq!(node.peer_count(), 1);
    }

    #[test]
    fn test_add_duplicate_peer_fails() {
        let mut node = make_node();
        let peer_id  = make_peer_id(1);
        node.add_peer(peer_id, "127.0.0.1:30303".into(), true).unwrap();
        let result = node.add_peer(peer_id, "127.0.0.1:30303".into(), true);
        assert!(matches!(result, Err(NetworkError::PeerAlreadyConnected)));
    }

    #[test]
    fn test_handle_hello_accepts_valid() {
        let mut node = make_node();
        let peer_id  = make_peer_id(1);
        node.add_peer(peer_id, "127.0.0.1:30303".into(), true).unwrap();

        let actions = node.handle_message(peer_id, valid_hello(5), Timestamp::now());
        let has_ack = actions.iter().any(|a| matches!(a,
            NetworkAction::SendMessage { msg: NetworkMessage::HelloAck { accepted: true, .. }, .. }
        ));
        assert!(has_ack, "should send HelloAck(accepted=true)");
    }

    #[test]
    fn test_handle_hello_triggers_sync_if_peer_ahead() {
        let mut node = make_node();
        let peer_id  = make_peer_id(2);
        node.add_peer(peer_id, "127.0.0.1:30304".into(), true).unwrap();

        // Peer is at height 100, we are at 0.
        let actions = node.handle_message(peer_id, valid_hello(100), Timestamp::now());
        let has_sync = actions.iter().any(|a| matches!(a, NetworkAction::RequestSync { .. }));
        assert!(has_sync, "should request sync when peer is ahead");
    }

    #[test]
    fn test_handle_message_before_handshake_disconnects() {
        let mut node = make_node();
        let peer_id  = make_peer_id(3);
        node.add_peer(peer_id, "127.0.0.1:30305".into(), true).unwrap();

        // Send Ping without completing handshake first.
        let actions = node.handle_message(peer_id, NetworkMessage::Ping { nonce: 1 }, Timestamp::now());
        let has_disconnect = actions.iter().any(|a| matches!(a, NetworkAction::DisconnectPeer { .. }));
        assert!(has_disconnect);
    }

    #[test]
    fn test_ping_pong() {
        let mut node = make_node();
        let peer_id  = make_peer_id(4);
        node.add_peer(peer_id, "127.0.0.1:30306".into(), true).unwrap();
        // Complete handshake.
        node.handle_message(peer_id, valid_hello(0), Timestamp::now());

        let actions = node.handle_message(peer_id, NetworkMessage::Ping { nonce: 99 }, Timestamp::now());
        let has_pong = actions.iter().any(|a| matches!(a,
            NetworkAction::SendMessage { msg: NetworkMessage::Pong { nonce: 99 }, .. }
        ));
        assert!(has_pong);
    }

    #[test]
    fn test_remove_peer() {
        let mut node = make_node();
        let peer_id  = make_peer_id(5);
        node.add_peer(peer_id, "127.0.0.1:30307".into(), false).unwrap();
        assert_eq!(node.peer_count(), 1);
        node.remove_peer(&peer_id);
        assert_eq!(node.peer_count(), 0);
    }

    // ── PeerScore ─────────────────────────────────────────────────────────────

    #[test]
    fn test_peer_score_ban_threshold() {
        let mut score = PeerScore::new();
        assert!(!score.is_banned());
        // Two invalid blocks should cross the ban threshold.
        score.apply(ScoreEvent::InvalidBlock); // -50
        score.apply(ScoreEvent::InvalidBlock); // -100
        assert!(score.is_banned());
    }

    #[test]
    fn test_peer_score_rewards() {
        let mut score = PeerScore::new();
        for _ in 0..5 {
            score.apply(ScoreEvent::ValidBlock); // +20 each
        }
        assert_eq!(score.value(), 100);
    }

    // ── RateLimiter ───────────────────────────────────────────────────────────

    #[test]
    fn test_rate_limiter_allows_under_limit() {
        let mut rl  = RateLimiter::new(1000, 10);
        let peer_id = make_peer_id(1);
        let now     = Timestamp::now();
        for _ in 0..10 {
            assert!(rl.check(&peer_id, now));
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let mut rl  = RateLimiter::new(1000, 5);
        let peer_id = make_peer_id(2);
        let now     = Timestamp::now();
        for _ in 0..5 { rl.check(&peer_id, now); }
        assert!(!rl.check(&peer_id, now), "should be rate-limited");
    }

    // ── BanList ───────────────────────────────────────────────────────────────

    #[test]
    fn test_ban_and_check() {
        let mut ban  = BanList::new();
        let peer_id  = make_peer_id(7);
        assert!(!ban.is_banned(&peer_id));
        ban.ban(&peer_id, "test ban");
        assert!(ban.is_banned(&peer_id));
        ban.unban(&peer_id);
        assert!(!ban.is_banned(&peer_id));
    }

    // ── BroadcastTracker ─────────────────────────────────────────────────────

    #[test]
    fn test_broadcast_tracker_deduplication() {
        let mut tracker = BroadcastTracker::new(100);
        let hash = sha256(b"block1");
        assert!(tracker.is_new_block(&hash));
        assert!(!tracker.is_new_block(&hash)); // duplicate
    }

    // ── HeaderSync ────────────────────────────────────────────────────────────

    #[test]
    fn test_header_sync_completes_on_short_response() {
        let tip_hash = sha256(b"tip");
        let mut sync = HeaderSync::new(tip_hash);

        // Simulate receiving fewer than MAX_HEADERS_PER_MSG headers → complete.
        let headers = vec![]; // 0 < 2000 → complete
        let done = sync.on_headers(headers);
        assert!(done);
    }

    // ── BlockSync ─────────────────────────────────────────────────────────────

    #[test]
    fn test_block_sync_pending_count() {
        use block::BlockBuilder;
        use primitives::Timestamp;
        use crypto::Address;

        let addr = Address::from_public_key(KeyPair::generate().unwrap().public_key());
        let block = BlockBuilder::new()
            .height(BlockHeight::new(1))
            .parent_hash(sha256(b"p"))
            .state_root(sha256(b"s"))
            .miner(addr)
            .difficulty(1)
            .timestamp(Timestamp::now())
            .build()
            .unwrap();

        let sync = BlockSync::from_headers(&[block.header().clone()]);
        assert_eq!(sync.pending_count(), 1);
        assert!(!sync.is_complete());
    }

    // ── FastSync ─────────────────────────────────────────────────────────────

    #[test]
    fn test_fast_sync_lifecycle() {
        let mut fs = FastSync::new();
        assert_eq!(fs.state, FastSyncState::Idle);
        fs.start(BlockHeight::new(1000));
        assert!(matches!(fs.state, FastSyncState::AwaitingSnapshot { .. }));
        fs.on_snapshot_received(BlockHeight::new(1000));
        assert!(matches!(fs.state, FastSyncState::Verifying { .. }));
        fs.on_verified(BlockHeight::new(1000));
        assert!(fs.is_complete());
    }

    // ── PeerDiscovery ─────────────────────────────────────────────────────────

    #[test]
    fn test_discovery_seeded_from_dns() {
        let disc = PeerDiscovery::new(100);
        // Should have at least the DNS seed addresses.
        assert!(disc.known_count() > 0);
    }

    #[test]
    fn test_discovery_add_addrs() {
        let mut disc = PeerDiscovery::new(100);
        let initial = disc.known_count();
        disc.add_addrs(vec!["192.168.1.1:30303".into()]);
        assert_eq!(disc.known_count(), initial + 1);
    }
}
