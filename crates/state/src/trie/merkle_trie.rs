// trie/merkle_trie.rs — Merkle root computation for world state.
//
// Purpose: produce a single 32-byte digest (state_root) that cryptographically
// commits to the entire world state (all account balances and nonces).
//
// Design:
//   • We use a sorted-leaf Merkle tree. Leaves are SHA-256(account_bytes)
//     sorted lexicographically by address checksum hex. This ensures the
//     root is deterministic regardless of HashMap iteration order.
//
//   • account_bytes = address(20) || balance_le8 || nonce_le8
//     This is fixed-width and unambiguous.
//
//   • The tree is built bottom-up:
//       leaf  = SHA-256(account_bytes)
//       node  = SHA-256(left_child || right_child)
//       root  = top node (or leaf if only one account)
//       empty = SHA-256(b"EMPTY_STATE")
//
//   • Domain prefix "SYPCOIN_STATE_LEAF_V1" is prepended to each leaf
//     pre-image to prevent second-preimage attacks where an internal node
//     could be mistaken for a leaf.
//
// NOTE: This is a simplified Merkle tree sufficient for state root computation.
// A full Merkle Patricia Trie (for inclusion proofs) would be added in a
// later iteration when light client support is needed.

use crypto::{HashDigest, sha256};
use crate::account::account::Account;

const LEAF_DOMAIN:  &[u8] = b"SYPCOIN_STATE_LEAF_V1";
const NODE_DOMAIN:  &[u8] = b"SYPCOIN_STATE_NODE_V1";
const EMPTY_DOMAIN: &[u8] = b"SYPCOIN_STATE_EMPTY_V1";

/// Compute the Merkle state root from a slice of accounts.
///
/// The result is deterministic: same set of accounts → same root,
/// regardless of the order they are passed in.
pub fn compute_state_root(accounts: &[&Account]) -> HashDigest {
    if accounts.is_empty() {
        return sha256(EMPTY_DOMAIN);
    }

    // 1. Sort accounts by address hex for determinism.
    let mut sorted: Vec<&&Account> = accounts.iter().collect();
    sorted.sort_by_key(|a| a.address().to_checksum_hex());

    // 2. Compute leaf hashes.
    let mut layer: Vec<HashDigest> = sorted
        .iter()
        .map(|a| leaf_hash(a))
        .collect();

    // 3. Build tree bottom-up until one root remains.
    while layer.len() > 1 {
        layer = layer
            .chunks(2)
            .map(|pair| {
                if pair.len() == 2 {
                    node_hash(&pair[0], &pair[1])
                } else {
                    // Odd number of nodes: promote the unpaired node.
                    pair[0].clone()
                }
            })
            .collect();
    }

    layer.into_iter().next()
        .unwrap_or_else(|| crypto::sha256(b"empty_state_trie"))
}

/// Compute the leaf hash for a single account.
///
/// Pre-image: LEAF_DOMAIN || address(20) || balance_le8 || nonce_le8
fn leaf_hash(account: &Account) -> HashDigest {
    let mut pre = Vec::with_capacity(LEAF_DOMAIN.len() + 20 + 8 + 8);
    pre.extend_from_slice(LEAF_DOMAIN);
    pre.extend_from_slice(account.address().as_bytes());
    pre.extend_from_slice(&account.balance().as_micro().to_le_bytes());
    pre.extend_from_slice(&account.nonce().as_u64().to_le_bytes());
    sha256(&pre)
}

/// Compute an internal node hash from two children.
///
/// Pre-image: NODE_DOMAIN || left(32) || right(32)
fn node_hash(left: &HashDigest, right: &HashDigest) -> HashDigest {
    let mut pre = Vec::with_capacity(NODE_DOMAIN.len() + 32 + 32);
    pre.extend_from_slice(NODE_DOMAIN);
    pre.extend_from_slice(left.as_bytes());
    pre.extend_from_slice(right.as_bytes());
    sha256(&pre)
}
