// merkle/tree.rs — Merkle root for the transaction list in a block.
//
// Security decisions:
//
//   SORTED BY TX_ID
//   • Leaves are sorted by tx_id (SHA-256 hash) before building the tree.
//     This makes the merkle_root independent of the order transactions
//     were added to the block, ensuring all nodes compute the same root.
//
//   DOMAIN-SEPARATED LEAVES
//   • leaf  = SHA-256(b"SYPCOIN_TX_LEAF_V1"  || tx_id_bytes)
//   • node  = SHA-256(b"SYPCOIN_TX_NODE_V1"  || left || right)
//   • empty = SHA-256(b"SYPCOIN_TX_EMPTY_V1")
//
//   SECOND-PREIMAGE RESISTANCE
//   • Different domain prefixes for leaf vs node hashes prevent an attacker
//     from constructing a fake proof by presenting an internal node as a leaf.
//     This closes the CVE-2012-2459 class of vulnerabilities (Bitcoin Merkle
//     tree second-preimage attack).

use crypto::{HashDigest, sha256};
use transaction::Transaction;

const TX_LEAF_DOMAIN:  &[u8] = b"SYPCOIN_TX_LEAF_V1";
const TX_NODE_DOMAIN:  &[u8] = b"SYPCOIN_TX_NODE_V1";
const TX_EMPTY_DOMAIN: &[u8] = b"SYPCOIN_TX_EMPTY_V1";

/// Compute the Merkle root of a list of transactions.
///
/// Returns a deterministic root regardless of transaction order.
/// Empty block → special empty-tree hash.
pub fn compute_merkle_root(transactions: &[Transaction]) -> HashDigest {
    if transactions.is_empty() {
        return sha256(TX_EMPTY_DOMAIN);
    }

    // 1. Compute and sort leaf hashes by tx_id for determinism.
    let mut leaves: Vec<HashDigest> = transactions
        .iter()
        .map(|tx| compute_leaf(tx))
        .collect();

    leaves.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));

    // 2. Build tree bottom-up.
    build_tree(leaves)
}

/// Compute the leaf hash for a single transaction.
///
/// Pre-image: TX_LEAF_DOMAIN || tx_id(32)
fn compute_leaf(tx: &Transaction) -> HashDigest {
    let mut pre = Vec::with_capacity(TX_LEAF_DOMAIN.len() + 32);
    pre.extend_from_slice(TX_LEAF_DOMAIN);
    pre.extend_from_slice(tx.tx_id().as_bytes());
    sha256(&pre)
}

/// Build a Merkle tree from a list of hashes, returning the root.
fn build_tree(mut layer: Vec<HashDigest>) -> HashDigest {
    while layer.len() > 1 {
        layer = layer
            .chunks(2)
            .map(|pair| {
                if pair.len() == 2 {
                    compute_node(&pair[0], &pair[1])
                } else {
                    // Odd number: duplicate the last node (Bitcoin convention).
                    compute_node(&pair[0], &pair[0])
                }
            })
            .collect();
    }
    layer.into_iter().next()
        .unwrap_or_else(|| crypto::sha256(b"empty_merkle_tree"))
}

/// Compute an internal node hash.
///
/// Pre-image: TX_NODE_DOMAIN || left(32) || right(32)
fn compute_node(left: &HashDigest, right: &HashDigest) -> HashDigest {
    let mut pre = Vec::with_capacity(TX_NODE_DOMAIN.len() + 32 + 32);
    pre.extend_from_slice(TX_NODE_DOMAIN);
    pre.extend_from_slice(left.as_bytes());
    pre.extend_from_slice(right.as_bytes());
    sha256(&pre)
}
