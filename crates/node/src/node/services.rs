// node/services.rs — Container for all node subsystem instances.

use consensus::Blockchain;
use event_bus::EventBus;
use execution::Executor;
use metrics::NodeMetrics;
use networking::NetworkNode;
use security::{AntiSpam, ReplayProtection, SignatureCache};
use state::WorldState;
use storage::Storage;
use transaction::Mempool;

/// All live subsystem instances owned by the node.
pub struct NodeServices {
    pub blockchain:        Blockchain,
    pub executor:          Executor,
    pub mempool:           Mempool,
    pub storage:           Storage,
    pub network:           NetworkNode,
    pub event_bus:         EventBus,
    pub metrics:           NodeMetrics,
    pub anti_spam:         AntiSpam,
    pub replay_protection: ReplayProtection,
    pub sig_cache:         SignatureCache,
}
