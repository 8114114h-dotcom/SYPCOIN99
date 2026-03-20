// publisher.rs — EventBus: the publishing side.
//
// Design:
//   • EventBus holds a Vec of mpsc::Sender<ChainEvent>.
//   • subscribe() creates a new (tx, rx) pair, stores tx, returns rx
//     wrapped in EventReceiver.
//   • publish() clones the event and sends it to every active subscriber.
//     If a subscriber's channel is full or disconnected, it is silently
//     dropped from the list (lazy cleanup).
//   • The bus is single-owner (not Clone). The node layer holds it behind
//     Arc<Mutex<EventBus>> if shared across threads.
//
// Capacity:
//   Each subscriber's channel has a bounded capacity (SUBSCRIBER_CAPACITY).
//   If a slow subscriber fills its buffer, events are dropped for that
//   subscriber only — other subscribers and the publisher are unaffected.
//   The dropped subscriber is removed from the list on the next publish.

use std::sync::mpsc;

use crate::events::ChainEvent;
use crate::subscriber::EventReceiver;

/// Per-subscriber channel buffer capacity.
const SUBSCRIBER_CAPACITY: usize = 1_024;

/// The central event bus — fan-out publisher to multiple subscribers.
pub struct EventBus {
    subscribers: Vec<mpsc::SyncSender<ChainEvent>>,
}

impl EventBus {
    /// Create a new, empty event bus.
    pub fn new() -> Self {
        EventBus { subscribers: Vec::new() }
    }

    /// Subscribe to all future events.
    ///
    /// Returns an `EventReceiver` that will receive every event published
    /// after this call. Events published before subscribe() are not delivered.
    pub fn subscribe(&mut self) -> EventReceiver {
        let (tx, rx) = mpsc::sync_channel(SUBSCRIBER_CAPACITY);
        self.subscribers.push(tx);
        EventReceiver::new(rx)
    }

    /// Publish an event to all active subscribers.
    ///
    /// The event is cloned for each subscriber. Disconnected or full
    /// subscribers are silently removed from the list.
    pub fn publish(&mut self, event: ChainEvent) {
        // Send to all subscribers, collecting indices of dead ones.
        let mut dead = Vec::new();
        for (i, tx) in self.subscribers.iter().enumerate() {
            // Clone only the Arc pointer, not the full event data
            match tx.try_send(event.clone()) {
                Ok(())                                      => {}
                Err(mpsc::TrySendError::Full(_))            => {
                    // Subscriber is too slow — drop this event for them.
                    // Could log here in production.
                }
                Err(mpsc::TrySendError::Disconnected(_))    => {
                    dead.push(i);
                }
            }
        }
        // Remove dead subscribers in reverse order to preserve indices.
        for i in dead.into_iter().rev() {
            self.subscribers.swap_remove(i);
        }
    }

    /// Number of currently active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }

    /// Returns `true` if there are no subscribers.
    pub fn is_empty(&self) -> bool {
        self.subscribers.is_empty()
    }
}

impl Default for EventBus {
    fn default() -> Self { Self::new() }
}
