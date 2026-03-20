// network.rs — P2P network configuration.

use serde::{Deserialize, Serialize};
use primitives::constants::{
    DEFAULT_P2P_PORT, MAX_INBOUND_PEERS, MAX_OUTBOUND_PEERS,
    PEER_CONNECT_TIMEOUT_MS, PEER_PING_INTERVAL_MS,
};

use crate::error::ConfigError;

/// P2P networking configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Address to listen on for incoming peer connections.
    /// Format: "ip:port"
    #[serde(default = "default_listen_addr")]
    pub listen_addr: String,

    /// External address announced to other peers (optional).
    /// Set this if behind NAT.
    #[serde(default)]
    pub external_addr: Option<String>,

    /// Maximum total peer connections (inbound + outbound).
    #[serde(default = "default_max_peers")]
    pub max_peers: usize,

    /// Maximum inbound (incoming) peer connections.
    #[serde(default = "default_max_inbound")]
    pub max_inbound: usize,

    /// Maximum outbound (initiated by us) peer connections.
    #[serde(default = "default_max_outbound")]
    pub max_outbound: usize,

    /// Bootstrap peers to connect to on startup.
    #[serde(default)]
    pub bootstrap_peers: Vec<String>,

    /// Interval between Ping messages to each peer (ms).
    #[serde(default = "default_ping_interval")]
    pub ping_interval_ms: u64,

    /// Timeout for establishing a peer connection (ms).
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_ms: u64,
}

impl NetworkConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.max_inbound + self.max_outbound > self.max_peers {
            return Err(ConfigError::InvalidValue {
                field:  "max_peers".into(),
                reason: "max_inbound + max_outbound must not exceed max_peers".into(),
            });
        }
        if self.ping_interval_ms == 0 {
            return Err(ConfigError::InvalidValue {
                field:  "ping_interval_ms".into(),
                reason: "must be > 0".into(),
            });
        }
        Ok(())
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        NetworkConfig {
            listen_addr:       default_listen_addr(),
            external_addr:     None,
            max_peers:         default_max_peers(),
            max_inbound:       default_max_inbound(),
            max_outbound:      default_max_outbound(),
            bootstrap_peers:   vec![],
            ping_interval_ms:  default_ping_interval(),
            connect_timeout_ms: default_connect_timeout(),
        }
    }
}

fn default_listen_addr()    -> String { format!("0.0.0.0:{}", DEFAULT_P2P_PORT) }
fn default_max_peers()      -> usize  { MAX_INBOUND_PEERS + MAX_OUTBOUND_PEERS }
fn default_max_inbound()    -> usize  { MAX_INBOUND_PEERS }
fn default_max_outbound()   -> usize  { MAX_OUTBOUND_PEERS }
fn default_ping_interval()  -> u64    { PEER_PING_INTERVAL_MS }
fn default_connect_timeout()-> u64    { PEER_CONNECT_TIMEOUT_MS }
