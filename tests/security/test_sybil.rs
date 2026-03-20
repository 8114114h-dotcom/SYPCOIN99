// security/test_sybil.rs
// Verify peer limits prevent Sybil attacks.

use crypto::sha256;
use networking::{NetworkConfig, NetworkNode, NetworkError};
use primitives::constants::{MAX_INBOUND_PEERS, MAX_OUTBOUND_PEERS};

fn make_peer_id(seed: u8) -> networking::PeerId {
    let mut id = [0u8; 32];
    id[0] = seed;
    id
}

fn make_node() -> NetworkNode {
    NetworkNode::new(NetworkConfig::default())
}

#[test]
fn test_max_inbound_peers_enforced() {
    let mut node = make_node();

    // Fill up inbound slots.
    for i in 0..MAX_INBOUND_PEERS {
        let peer_id = make_peer_id(i as u8);
        let result  = node.add_peer(peer_id, format!("10.0.0.{}:30303", i), true);
        assert!(result.is_ok(), "peer {} should be accepted", i);
    }

    // Next inbound peer must be rejected.
    let overflow_id = make_peer_id(255);
    let result = node.add_peer(overflow_id, "10.0.1.0:30303".into(), true);
    assert!(matches!(result, Err(NetworkError::MaxPeersReached)),
        "inbound peer beyond limit must be rejected");
}

#[test]
fn test_max_outbound_peers_enforced() {
    let mut node = make_node();

    for i in 0..MAX_OUTBOUND_PEERS {
        let peer_id = make_peer_id(i as u8);
        node.add_peer(peer_id, format!("10.0.2.{}:30303", i), false).unwrap();
    }

    let overflow_id = make_peer_id(254);
    let result = node.add_peer(overflow_id, "10.0.3.0:30303".into(), false);
    assert!(matches!(result, Err(NetworkError::MaxPeersReached)));
}

#[test]
fn test_banned_peer_rejected_immediately() {
    use networking::BanList;

    let mut ban_list = BanList::new();
    let peer_id      = make_peer_id(42);
    ban_list.ban(&peer_id, "sybil attack detected");

    assert!(ban_list.is_banned(&peer_id));
    // Different peer not affected.
    assert!(!ban_list.is_banned(&make_peer_id(43)));
}

#[test]
fn test_duplicate_peer_rejected() {
    let mut node    = make_node();
    let peer_id     = make_peer_id(10);
    node.add_peer(peer_id, "1.2.3.4:30303".into(), true).unwrap();

    let result = node.add_peer(peer_id, "1.2.3.4:30303".into(), true);
    assert!(matches!(result, Err(NetworkError::PeerAlreadyConnected)));
}

#[test]
fn test_peer_score_triggers_ban() {
    use networking::PeerScore;
    use networking::ScoreEvent;
    use networking::BanList;
    use security::AntiSpam;

    let mut score = PeerScore::new();
    assert!(!score.is_banned());

    // Two invalid blocks → score drops to -100 → banned.
    score.apply(ScoreEvent::InvalidBlock); // -50
    score.apply(ScoreEvent::InvalidBlock); // -100
    assert!(score.is_banned());
}

#[test]
fn test_rate_limiter_prevents_flood() {
    use networking::RateLimiter;
    use primitives::Timestamp;

    let mut rl      = RateLimiter::new(1000, 10);
    let peer_id     = make_peer_id(99);
    let now         = Timestamp::now();

    for _ in 0..10 { rl.check(&peer_id, now); }
    assert!(!rl.check(&peer_id, now), "flood beyond limit must be blocked");
}

#[test]
fn test_rate_limiter_different_peers_independent() {
    use networking::RateLimiter;
    use primitives::Timestamp;

    let mut rl  = RateLimiter::new(1000, 2);
    let peer_a  = make_peer_id(1);
    let peer_b  = make_peer_id(2);
    let now     = Timestamp::now();

    rl.check(&peer_a, now);
    rl.check(&peer_a, now);
    assert!(!rl.check(&peer_a, now), "peer A should be rate-limited");

    // peer_b is unaffected.
    assert!(rl.check(&peer_b, now), "peer B should not be affected by peer A's limit");
}
