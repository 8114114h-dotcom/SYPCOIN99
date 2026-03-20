// sync/header_sync.rs — Header-first synchronisation.
//
// Phase 1 of sync: download headers to find the common ancestor and
// determine how far behind we are, before downloading full blocks.

use block::BlockHeader;
use crypto::HashDigest;

use crate::protocol::messages::{NetworkMessage, MAX_HEADERS_PER_MSG};

/// State for an in-progress header sync session.
#[derive(Debug)]
pub struct HeaderSync {
    pub from_hash:    HashDigest,
    pub headers_recv: Vec<BlockHeader>,
    pub complete:     bool,
}

impl HeaderSync {
    pub fn new(tip_hash: HashDigest) -> Self {
        HeaderSync {
            from_hash:    tip_hash,
            headers_recv: Vec::new(),
            complete:     false,
        }
    }

    /// Build the next GetHeaders request.
    pub fn next_request(&self) -> NetworkMessage {
        let from = self.headers_recv
            .last()
            .map(|h| h.hash())
            .unwrap_or_else(|| self.from_hash.clone());

        NetworkMessage::GetHeaders {
            from_hash: from,
            limit:     MAX_HEADERS_PER_MSG,
        }
    }

    /// Process an incoming batch of headers.
    ///
    /// Returns `true` if sync is complete (peer sent fewer than limit).
    pub fn on_headers(&mut self, headers: Vec<BlockHeader>) -> bool {
        let received = headers.len();
        self.headers_recv.extend(headers);
        if (received as u32) < MAX_HEADERS_PER_MSG {
            self.complete = true;
        }
        self.complete
    }

    pub fn pending_headers(&self) -> &[BlockHeader] {
        &self.headers_recv
    }
}
