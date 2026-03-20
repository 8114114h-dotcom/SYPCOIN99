// lib.rs — Public API for the event_bus crate.
//
//   use event_bus::{EventBus, EventReceiver, ChainEvent};

mod error;
mod events;
mod publisher;
mod subscriber;

pub use error::EventBusError;
pub use events::ChainEvent;
pub use publisher::EventBus;
pub use subscriber::EventReceiver;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crypto::sha256;
    use primitives::{BlockHeight, Timestamp};

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn block_added_event(height: u64) -> ChainEvent {
        ChainEvent::BlockAdded {
            block_hash: sha256(&height.to_le_bytes()),
            height:     BlockHeight::new(height),
            tx_count:   0,
            timestamp:  Timestamp::now(),
        }
    }

    // ── Basic publish/subscribe ───────────────────────────────────────────────

    #[test]
    fn test_subscribe_and_receive() {
        let mut bus = EventBus::new();
        let rx      = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 1);

        bus.publish(block_added_event(1));

        let event = rx.try_recv().unwrap();
        assert!(matches!(event, ChainEvent::BlockAdded { .. }));
    }

    #[test]
    fn test_multiple_subscribers_all_receive() {
        let mut bus = EventBus::new();
        let rx1     = bus.subscribe();
        let rx2     = bus.subscribe();
        let rx3     = bus.subscribe();

        bus.publish(ChainEvent::NodeStarted {
            height:   BlockHeight::new(0),
            tip_hash: sha256(b"genesis"),
        });

        assert!(matches!(rx1.try_recv(), Some(ChainEvent::NodeStarted { .. })));
        assert!(matches!(rx2.try_recv(), Some(ChainEvent::NodeStarted { .. })));
        assert!(matches!(rx3.try_recv(), Some(ChainEvent::NodeStarted { .. })));
    }

    #[test]
    fn test_no_subscribers_publish_ok() {
        let mut bus = EventBus::new();
        // Publishing with no subscribers should not panic.
        bus.publish(ChainEvent::MiningStopped);
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[test]
    fn test_events_ordered() {
        let mut bus = EventBus::new();
        let rx      = bus.subscribe();

        for i in 1..=5 {
            bus.publish(block_added_event(i));
        }

        let events = rx.drain();
        assert_eq!(events.len(), 5);

        // Events should arrive in publish order.
        for (i, event) in events.iter().enumerate() {
            if let ChainEvent::BlockAdded { height, .. } = event {
                assert_eq!(height.as_u64(), (i + 1) as u64);
            } else {
                panic!("unexpected event type");
            }
        }
    }

    // ── Disconnected subscriber cleanup ───────────────────────────────────────

    #[test]
    fn test_dead_subscriber_removed_on_publish() {
        let mut bus = EventBus::new();
        let rx      = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 1);

        // Drop the receiver — subscriber is now dead.
        drop(rx);

        // Next publish should silently remove the dead subscriber.
        bus.publish(ChainEvent::MiningStopped);
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[test]
    fn test_live_subscribers_unaffected_by_dead_one() {
        let mut bus  = EventBus::new();
        let rx_live  = bus.subscribe();
        let rx_dead  = bus.subscribe();

        drop(rx_dead); // kill one subscriber

        bus.publish(ChainEvent::NodeStopping);

        // Live subscriber still gets the event.
        assert!(matches!(rx_live.try_recv(), Some(ChainEvent::NodeStopping)));
        assert_eq!(bus.subscriber_count(), 1);
    }

    // ── try_recv / recv / drain ───────────────────────────────────────────────

    #[test]
    fn test_try_recv_empty() {
        let mut bus = EventBus::new();
        let rx      = bus.subscribe();
        assert!(rx.try_recv().is_none());
    }

    #[test]
    fn test_drain_empty() {
        let mut bus = EventBus::new();
        let rx      = bus.subscribe();
        assert!(rx.drain().is_empty());
    }

    #[test]
    fn test_drain_multiple() {
        let mut bus = EventBus::new();
        let rx      = bus.subscribe();

        bus.publish(ChainEvent::MiningStopped);
        bus.publish(ChainEvent::NodeStopping);
        bus.publish(block_added_event(10));

        let drained = rx.drain();
        assert_eq!(drained.len(), 3);
        assert_eq!(drained[2].type_name(), "BlockAdded");
    }

    // ── Event type names ──────────────────────────────────────────────────────

    #[test]
    fn test_event_type_names() {
        let events: Vec<(ChainEvent, &str)> = vec![
            (block_added_event(1),                          "BlockAdded"),
            (ChainEvent::MiningStopped,                     "MiningStopped"),
            (ChainEvent::NodeStopping,                      "NodeStopping"),
            (ChainEvent::SyncCompleted { height: BlockHeight::new(5) }, "SyncCompleted"),
            (ChainEvent::PeerConnected { peer_addr: "1.2.3.4:30303".into(), peer_height: 0 }, "PeerConnected"),
        ];
        for (event, expected) in events {
            assert_eq!(event.type_name(), expected);
        }
    }

    // ── All event variants constructable ─────────────────────────────────────

    #[test]
    fn test_all_event_variants() {
        let hash = sha256(b"test");

        let _ = ChainEvent::BlockAdded         { block_hash: hash.clone(), height: BlockHeight::new(1), tx_count: 0, timestamp: Timestamp::now() };
        let _ = ChainEvent::BlockReverted      { block_hash: hash.clone(), height: BlockHeight::new(1) };
        let _ = ChainEvent::ChainReorganized   { old_tip: hash.clone(), new_tip: hash.clone(), depth: 3 };
        let _ = ChainEvent::NewTransaction     { tx_id: hash.clone(), from: "0x01".into(), to: "0x02".into(), amount: 1000 };
        let _ = ChainEvent::TransactionConfirmed { tx_id: hash.clone(), block_height: BlockHeight::new(1), block_hash: hash.clone() };
        let _ = ChainEvent::TransactionEvicted { tx_id: hash.clone(), reason: "expired".into() };
        let _ = ChainEvent::MiningStarted      { height: BlockHeight::new(2), difficulty: 1000 };
        let _ = ChainEvent::MiningStopped;
        let _ = ChainEvent::BlockMined         { block_hash: hash.clone(), height: BlockHeight::new(2), nonce: 42, elapsed_ms: 1500 };
        let _ = ChainEvent::PeerConnected      { peer_addr: "1.2.3.4:30303".into(), peer_height: 10 };
        let _ = ChainEvent::PeerDisconnected   { peer_addr: "1.2.3.4:30303".into(), reason: "timeout".into() };
        let _ = ChainEvent::SyncStarted        { from_height: 0, to_height: 100, peer_addr: "1.2.3.4:30303".into() };
        let _ = ChainEvent::SyncCompleted      { height: BlockHeight::new(100) };
        let _ = ChainEvent::NodeStarted        { height: BlockHeight::new(0), tip_hash: hash.clone() };
        let _ = ChainEvent::NodeStopping;
    }
}
