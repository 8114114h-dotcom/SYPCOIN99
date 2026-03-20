// node/shutdown.rs — Graceful shutdown coordination.
//
// ShutdownHandle is shared across threads. When trigger() is called
// (e.g. on SIGINT), all components watching the signal stop cleanly.

use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

/// Shared shutdown flag.
#[derive(Clone)]
pub struct ShutdownHandle {
    flag: Arc<AtomicBool>,
}

impl ShutdownHandle {
    pub fn new() -> Self {
        ShutdownHandle {
            flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Signal all watchers to shut down.
    pub fn trigger(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    /// Returns `true` if shutdown has been triggered.
    pub fn is_triggered(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

impl Default for ShutdownHandle {
    fn default() -> Self { Self::new() }
}
