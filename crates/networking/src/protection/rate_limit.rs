// protection/rate_limit.rs — Per-peer message rate limiting.
//
// Uses a sliding window counter per peer.
// If a peer sends more than MAX_MESSAGES_PER_WINDOW messages within
// WINDOW_MS milliseconds, further messages are dropped and the peer
// is penalised.

use std::collections::HashMap;

use primitives::Timestamp;

use crate::error::PeerId;

/// Time window in milliseconds.
pub const WINDOW_MS: u64 = 1_000; // 1 second

/// Maximum messages allowed per peer per window.
pub const MAX_MESSAGES_PER_WINDOW: u32 = 100;

#[derive(Debug)]
struct Window {
    count:      u32,
    window_start_ms: u64,
}

/// Sliding-window rate limiter.
pub struct RateLimiter {
    windows:    HashMap<String, Window>,
    window_ms:  u64,
    max_msgs:   u32,
}

impl RateLimiter {
    pub fn new(window_ms: u64, max_msgs: u32) -> Self {
        RateLimiter {
            windows:   HashMap::new(),
            window_ms,
            max_msgs,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(WINDOW_MS, MAX_MESSAGES_PER_WINDOW)
    }

    /// Check if a peer is allowed to send another message.
    ///
    /// Returns `true` if allowed, `false` if rate-limited.
    pub fn check(&mut self, peer_id: &PeerId, now: Timestamp) -> bool {
        let key = hex::encode(peer_id);
        let now_ms = now.as_millis();

        let window = self.windows.entry(key).or_insert(Window {
            count:           0,
            window_start_ms: now_ms,
        });

        // Reset window if expired.
        if now_ms.saturating_sub(window.window_start_ms) >= self.window_ms {
            window.count = 0;
            window.window_start_ms = now_ms;
        }

        if window.count >= self.max_msgs {
            return false; // rate limited
        }

        window.count += 1;
        true
    }

    /// Remove the tracking entry for a disconnected peer.
    pub fn remove(&mut self, peer_id: &PeerId) {
        self.windows.remove(&hex::encode(peer_id));
    }

    /// Clear all tracking data.
    pub fn clear(&mut self) {
        self.windows.clear();
    }
}
