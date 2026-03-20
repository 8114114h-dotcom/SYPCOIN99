// mempool/ordering.rs — Fee-based ordering key for the mempool BTreeMap.
//
// Design:
//   FeeKey = (Reverse(fee_micro), tx_id_bytes[0..8])
//
//   • Reverse(fee_micro) ensures highest-fee transactions sort FIRST in
//     BTreeMap iteration (BTreeMap is ascending by default).
//   • tx_id_bytes[0..8] breaks ties deterministically across nodes.
//     Using the tx_id prefix (which is a SHA-256 hash) gives uniform
//     distribution in tie-breaking without bias.
//
// This ordering is used only for mempool selection — it does NOT affect
// consensus (the block producer may choose their own ordering within the
// block, subject to the nonce sequence constraint per address).

use std::cmp::Reverse;
use crate::tx::transaction::Transaction;

/// Ordering key for the fee-sorted mempool index.
///
/// Sorts highest-fee transactions first. Ties broken by tx_id prefix.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct FeeKey {
    /// Reversed so that higher fees sort first in BTreeMap.
    fee_desc: Reverse<u64>,
    /// First 8 bytes of tx_id for deterministic tie-breaking.
    tx_id_prefix: [u8; 8],
}

impl FeeKey {
    /// Construct a FeeKey from a transaction.
    pub fn new(tx: &Transaction) -> Self {
        let mut prefix = [0u8; 8];
        prefix.copy_from_slice(&tx.tx_id().as_bytes()[..8]);
        FeeKey {
            fee_desc:     Reverse(tx.fee().as_micro()),
            tx_id_prefix: prefix,
        }
    }
}
