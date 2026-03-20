// error.rs

use thiserror::Error;

#[derive(Debug, Error)]
pub enum EventBusError {
    #[error("event bus is closed — all subscribers have dropped")]
    BusClosed,

    #[error("subscriber channel is full — event dropped")]
    ChannelFull,
}
