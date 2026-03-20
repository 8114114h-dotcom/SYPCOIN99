// gossip/router.rs — Message routing decisions.
//
// The router decides which peers should receive a given message,
// applying gossip fan-out rules.

use crate::error::PeerId;

/// Routing strategy for a message.
pub enum RouteTarget {
    /// Send to all connected peers.
    All,
    /// Send to all peers except the source.
    AllExcept(PeerId),
    /// Send to a specific peer.
    Specific(PeerId),
    /// Drop the message (already seen or not relevant).
    Drop,
}

/// Determine the routing target for a message.
///
/// `source` — the peer the message came from (None if locally generated).
pub fn route_message(source: Option<PeerId>) -> RouteTarget {
    match source {
        None         => RouteTarget::All,
        Some(src_id) => RouteTarget::AllExcept(src_id),
    }
}
