// subscriber.rs — EventReceiver: the consumer side of the event bus.

use std::sync::mpsc;

use crate::events::ChainEvent;

/// The receiving end of an event subscription.
///
/// Each subscriber gets its own `EventReceiver` with an independent buffer.
/// A slow subscriber does not block other subscribers or the publisher.
pub struct EventReceiver {
    rx: mpsc::Receiver<ChainEvent>,
}

impl EventReceiver {
    pub(crate) fn new(rx: mpsc::Receiver<ChainEvent>) -> Self {
        EventReceiver { rx }
    }

    /// Block until an event arrives or the bus is closed.
    ///
    /// Returns `None` if the bus has been dropped (node is shutting down).
    pub fn recv(&self) -> Option<ChainEvent> {
        self.rx.recv().ok()
    }

    /// Non-blocking check for a pending event.
    ///
    /// Returns `None` if no event is currently available.
    pub fn try_recv(&self) -> Option<ChainEvent> {
        self.rx.try_recv().ok()
    }

    /// Drain all currently available events without blocking.
    ///
    /// Useful for batch processing in a polling loop.
    pub fn drain(&self) -> Vec<ChainEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.rx.try_recv() {
            events.push(event);
        }
        events
    }
}
