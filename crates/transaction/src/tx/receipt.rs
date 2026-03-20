// tx/receipt.rs — Transaction execution receipt.
//
// A receipt is produced by the state transition layer after a transaction
// is applied to the world state. It records:
//   - Whether the transaction succeeded or failed.
//   - The block it was included in.
//   - The fee that was paid to the block producer.
//
// Receipts are stored alongside blocks and returned by the RPC layer
// when a client queries a transaction by its tx_id.

use serde::{Deserialize, Serialize};

use crypto::HashDigest;
use primitives::{Amount, BlockHeight, Timestamp};

/// The outcome of executing a transaction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReceiptStatus {
    /// Transaction was applied successfully.
    Success,
    /// Transaction failed during execution. The fee is still consumed.
    Failed(String),
}

impl ReceiptStatus {
    /// Returns `true` if the transaction succeeded.
    pub fn is_success(&self) -> bool {
        matches!(self, ReceiptStatus::Success)
    }

    /// Returns the failure reason, if any.
    pub fn failure_reason(&self) -> Option<&str> {
        match self {
            ReceiptStatus::Failed(reason) => Some(reason),
            ReceiptStatus::Success        => None,
        }
    }
}

impl std::fmt::Display for ReceiptStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReceiptStatus::Success        => write!(f, "Success"),
            ReceiptStatus::Failed(reason) => write!(f, "Failed: {}", reason),
        }
    }
}

/// A record of a transaction's execution produced by the state layer.
///
/// Stored in the block database alongside the block that included the tx.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionReceipt {
    /// The transaction that was executed.
    pub tx_id: HashDigest,

    /// Block that included this transaction.
    pub block_height: BlockHeight,

    /// Hash of the block that included this transaction.
    pub block_hash: HashDigest,

    /// Execution outcome.
    pub status: ReceiptStatus,

    /// Fee actually paid to the block producer.
    /// Equal to `tx.fee()` for standard transactions.
    pub fee_paid: Amount,

    /// Timestamp of the block that included this transaction.
    pub block_timestamp: Timestamp,
}

impl TransactionReceipt {
    /// Construct a success receipt.
    pub fn success(
        tx_id:           HashDigest,
        block_height:    BlockHeight,
        block_hash:      HashDigest,
        fee_paid:        Amount,
        block_timestamp: Timestamp,
    ) -> Self {
        TransactionReceipt {
            tx_id,
            block_height,
            block_hash,
            status: ReceiptStatus::Success,
            fee_paid,
            block_timestamp,
        }
    }

    /// Construct a failure receipt.
    pub fn failed(
        tx_id:           HashDigest,
        block_height:    BlockHeight,
        block_hash:      HashDigest,
        fee_paid:        Amount,
        block_timestamp: Timestamp,
        reason:          String,
    ) -> Self {
        TransactionReceipt {
            tx_id,
            block_height,
            block_hash,
            status: ReceiptStatus::Failed(reason),
            fee_paid,
            block_timestamp,
        }
    }

    /// Returns `true` if the transaction was executed successfully.
    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }
}

impl std::fmt::Display for TransactionReceipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Receipt[{}] @ {} | {} | fee: {}",
            hex::encode(&self.tx_id.as_bytes()[..6]),
            self.block_height,
            self.status,
            self.fee_paid,
        )
    }
}
