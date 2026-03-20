// middleware.rs — RPC middleware: request validation and rate limiting.

use std::collections::HashMap;
use primitives::Timestamp;

/// Maximum RPC requests per IP per second.
pub const MAX_RPC_REQUESTS_PER_SEC: u32 = 50;

/// Per-IP rate limiter for the RPC server.
pub struct RpcRateLimiter {
    windows:  HashMap<String, (u32, u64)>,  // ip → (count, window_start_ms)
    max_reqs: u32,
    window_ms: u64,
}

impl RpcRateLimiter {
    pub fn new() -> Self {
        RpcRateLimiter {
            windows:   HashMap::new(),
            max_reqs:  MAX_RPC_REQUESTS_PER_SEC,
            window_ms: 1_000,
        }
    }

    /// Returns `true` if the request is allowed.
    pub fn check(&mut self, ip: &str) -> bool {
        let now_ms = Timestamp::now().as_millis();
        let entry  = self.windows.entry(ip.to_owned()).or_insert((0, now_ms));

        if now_ms.saturating_sub(entry.1) >= self.window_ms {
            *entry = (0, now_ms);
        }

        if entry.0 >= self.max_reqs {
            return false;
        }
        entry.0 += 1;
        true
    }

    pub fn cleanup(&mut self) {
        let now_ms = Timestamp::now().as_millis();
        self.windows.retain(|_, (_, start)| {
            now_ms.saturating_sub(*start) < self.window_ms * 10
        });
    }
}

impl Default for RpcRateLimiter {
    fn default() -> Self { Self::new() }
}
