// protection/dns_seeds.rs — Bootstrap peer addresses (DNS seeds).
//
// When a node starts fresh with no known peers, it uses these hardcoded
// addresses to find its first connections.
//
// In production these would be DNS names that resolve to multiple IPs.
// For now we use static IP:port pairs as placeholders.

/// Hardcoded bootstrap peer addresses.
/// Replace with actual seed nodes before mainnet launch.
pub const DNS_SEEDS: &[&str] = &[
    "seed1.sypcoin.network:30303",
    "seed2.sypcoin.network:30303",
    "seed3.sypcoin.network:30303",
];

/// Return the list of seed addresses as owned strings.
pub fn get_seeds() -> Vec<String> {
    DNS_SEEDS.iter().map(|s| s.to_string()).collect()
}
