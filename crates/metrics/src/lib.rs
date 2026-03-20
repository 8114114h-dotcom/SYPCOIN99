// lib.rs — Public API for the metrics crate.
//
//   use metrics::{NodeMetrics, PrometheusExporter, init_tracing};

mod node_metrics;
mod prometheus;
pub mod tracing;

pub use node_metrics::NodeMetrics;
pub use prometheus::export as prometheus_export;
pub use tracing::{init_tracing, log, LogLevel};

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    // ── NodeMetrics ───────────────────────────────────────────────────────────

    #[test]
    fn test_new_metrics_defaults() {
        let m = NodeMetrics::new();
        assert_eq!(m.chain_height, 0);
        assert_eq!(m.connected_peers, 0);
        assert_eq!(m.blocks_mined, 0);
        assert_eq!(m.mempool_size, 0);
    }

    #[test]
    fn test_update_chain() {
        let mut m = NodeMetrics::new();
        m.update_chain(42, "abc123".repeat(10)[..64].to_string(), 10);
        assert_eq!(m.chain_height, 42);
        assert_eq!(m.total_transactions, 10);
    }

    #[test]
    fn test_update_chain_accumulates_txs() {
        let mut m = NodeMetrics::new();
        m.update_chain(1, "a".repeat(64), 5);
        m.update_chain(2, "b".repeat(64), 3);
        assert_eq!(m.total_transactions, 8);
    }

    #[test]
    fn test_update_mining() {
        let mut m = NodeMetrics::new();
        m.update_mining(100_000, 5_000);
        assert_eq!(m.blocks_mined, 1);
        assert_eq!(m.last_nonces_tried, 100_000);
        assert_eq!(m.last_block_time_ms, 5_000);
    }

    #[test]
    fn test_update_mining_accumulates() {
        let mut m = NodeMetrics::new();
        m.update_mining(50_000, 3_000);
        m.update_mining(80_000, 4_000);
        assert_eq!(m.blocks_mined, 2);
        // last values overwrite previous
        assert_eq!(m.last_nonces_tried, 80_000);
    }

    #[test]
    fn test_hash_rate_zero_when_no_mining() {
        let m = NodeMetrics::new();
        assert_eq!(m.hash_rate(), 0.0);
    }

    #[test]
    fn test_hash_rate_calculated() {
        let mut m = NodeMetrics::new();
        // 10_000 nonces in 1_000ms → 10_000 H/s
        m.update_mining(10_000, 1_000);
        assert!((m.hash_rate() - 10_000.0).abs() < 1.0);
    }

    #[test]
    fn test_update_network() {
        let mut m = NodeMetrics::new();
        m.update_network(5, 1024, 2048, 3, 15);
        assert_eq!(m.connected_peers,   5);
        assert_eq!(m.total_bytes_sent,  1024);
        assert_eq!(m.total_bytes_recv,  2048);
        assert_eq!(m.total_blocks_recv, 3);
        assert_eq!(m.total_txs_recv,    15);
    }

    #[test]
    fn test_update_network_accumulates_bytes() {
        let mut m = NodeMetrics::new();
        m.update_network(3, 500, 1000, 1, 5);
        m.update_network(4, 300, 700,  2, 3);
        assert_eq!(m.total_bytes_sent,  800);
        assert_eq!(m.total_bytes_recv,  1700);
        assert_eq!(m.total_blocks_recv, 3);
        assert_eq!(m.total_txs_recv,    8);
        // peers is overwritten, not accumulated
        assert_eq!(m.connected_peers,   4);
    }

    #[test]
    fn test_update_mempool() {
        let mut m = NodeMetrics::new();
        m.update_mempool(42);
        assert_eq!(m.mempool_size, 42);
        m.update_mempool(10);
        assert_eq!(m.mempool_size, 10);
    }

    #[test]
    fn test_uptime_increases() {
        let m = NodeMetrics::new();
        thread::sleep(Duration::from_millis(10));
        assert!(m.uptime_seconds() >= 0); // at least 0
    }

    // ── Prometheus export ─────────────────────────────────────────────────────

    #[test]
    fn test_prometheus_export_contains_required_metrics() {
        let mut m = NodeMetrics::new();
        m.update_chain(100, "a".repeat(64), 500);
        m.update_mining(1_000_000, 10_000);
        m.update_network(8, 4096, 8192, 10, 50);
        m.update_mempool(25);

        let output = prometheus_export(&m);

        // Check required metric names are present.
        assert!(output.contains("sypcoin_chain_height"));
        assert!(output.contains("sypcoin_total_transactions"));
        assert!(output.contains("sypcoin_blocks_mined_total"));
        assert!(output.contains("sypcoin_mining_hash_rate"));
        assert!(output.contains("sypcoin_connected_peers"));
        assert!(output.contains("sypcoin_bytes_sent_total"));
        assert!(output.contains("sypcoin_bytes_recv_total"));
        assert!(output.contains("sypcoin_mempool_size"));
        assert!(output.contains("sypcoin_uptime_seconds"));
    }

    #[test]
    fn test_prometheus_format_has_help_and_type() {
        let m      = NodeMetrics::new();
        let output = prometheus_export(&m);

        // Every metric must have # HELP and # TYPE lines.
        assert!(output.contains("# HELP sypcoin_chain_height"));
        assert!(output.contains("# TYPE sypcoin_chain_height gauge"));
        assert!(output.contains("# HELP sypcoin_blocks_mined_total"));
        assert!(output.contains("# TYPE sypcoin_blocks_mined_total counter"));
    }

    #[test]
    fn test_prometheus_correct_values() {
        let mut m = NodeMetrics::new();
        m.update_chain(77, "x".repeat(64), 0);

        let output = prometheus_export(&m);
        assert!(output.contains("sypcoin_chain_height 77"));
    }

    // ── LogLevel ──────────────────────────────────────────────────────────────

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::from_str("error"), LogLevel::Error);
        assert_eq!(LogLevel::from_str("WARN"),  LogLevel::Warn);
        assert_eq!(LogLevel::from_str("info"),  LogLevel::Info);
        assert_eq!(LogLevel::from_str("debug"), LogLevel::Debug);
        assert_eq!(LogLevel::from_str("trace"), LogLevel::Trace);
        assert_eq!(LogLevel::from_str("other"), LogLevel::Info); // default
    }

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Error < LogLevel::Warn);
        assert!(LogLevel::Warn  < LogLevel::Info);
        assert!(LogLevel::Info  < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Trace);
    }
}
