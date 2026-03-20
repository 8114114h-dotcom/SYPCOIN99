// prometheus.rs — Prometheus text format exporter.
//
// Produces the standard Prometheus exposition format:
//   # HELP <name> <description>
//   # TYPE <name> <type>
//   <name> <value>
//
// Served at the /metrics HTTP endpoint. Prometheus scrapes it every 15s.
// The format is plain text — no external crate needed.

use crate::node_metrics::NodeMetrics;

/// Export all metrics in Prometheus text format.
pub fn export(metrics: &NodeMetrics) -> String {
    let mut out = String::with_capacity(2048);

    // ── Chain ─────────────────────────────────────────────────────────────────
    gauge(&mut out, "sypcoin_chain_height",
        "Current canonical chain height (blocks)",
        metrics.chain_height as f64);

    counter(&mut out, "sypcoin_total_transactions",
        "Total transactions confirmed on-chain",
        metrics.total_transactions as f64);

    // ── Mining ────────────────────────────────────────────────────────────────
    counter(&mut out, "sypcoin_blocks_mined_total",
        "Total blocks mined by this node",
        metrics.blocks_mined as f64);

    gauge(&mut out, "sypcoin_mining_hash_rate",
        "Approximate mining hash rate (hashes/sec) from last block",
        metrics.hash_rate());

    gauge(&mut out, "sypcoin_last_block_time_ms",
        "Time to mine the last block in milliseconds",
        metrics.last_block_time_ms as f64);

    // ── Network ───────────────────────────────────────────────────────────────
    gauge(&mut out, "sypcoin_connected_peers",
        "Number of currently connected peers",
        metrics.connected_peers as f64);

    counter(&mut out, "sypcoin_bytes_sent_total",
        "Total bytes sent to peers",
        metrics.total_bytes_sent as f64);

    counter(&mut out, "sypcoin_bytes_recv_total",
        "Total bytes received from peers",
        metrics.total_bytes_recv as f64);

    counter(&mut out, "sypcoin_blocks_recv_total",
        "Total blocks received from the network",
        metrics.total_blocks_recv as f64);

    counter(&mut out, "sypcoin_txs_recv_total",
        "Total transactions received from the network",
        metrics.total_txs_recv as f64);

    // ── Mempool ───────────────────────────────────────────────────────────────
    gauge(&mut out, "sypcoin_mempool_size",
        "Current number of pending transactions in the mempool",
        metrics.mempool_size as f64);

    // ── System ────────────────────────────────────────────────────────────────
    counter(&mut out, "sypcoin_uptime_seconds",
        "Seconds since the node started",
        metrics.uptime_seconds() as f64);

    out
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Append a gauge metric (can go up or down).
fn gauge(out: &mut String, name: &str, help: &str, value: f64) {
    out.push_str(&format!(
        "# HELP {name} {help}\n# TYPE {name} gauge\n{name} {value}\n",
        name = name, help = help, value = value
    ));
}

/// Append a counter metric (only increases).
fn counter(out: &mut String, name: &str, help: &str, value: f64) {
    out.push_str(&format!(
        "# HELP {name} {help}\n# TYPE {name} counter\n{name} {value}\n",
        name = name, help = help, value = value
    ));
}
