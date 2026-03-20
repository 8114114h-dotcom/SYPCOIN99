// handlers/tx.rs — Transaction handlers.

use crate::error::{RpcError, RpcResult};
use crate::server::http::RpcContext;
use crate::types::requests::RpcRequest;
use crate::types::responses::TxResponse;

/// getTransaction(tx_id: String) → TxResponse
pub fn get_transaction(ctx: &mut RpcContext, req: &RpcRequest) -> RpcResult {
    let tx_id_hex = req.require_string(0, "tx_id")?;
    let hash      = parse_hash(&tx_id_hex)?;

    match ctx.storage.get_transaction(&hash)
        .map_err(|e| RpcError::internal(&e.to_string()))?
    {
        None         => Err(RpcError::not_found("transaction")),
        Some(record) => Ok(serde_json::to_value(TxResponse::from_record(&record))
            .map_err(|e| RpcError::internal(&e.to_string()))?),
    }
}

/// sendTransaction(tx_hex: String) → tx_id String
///
/// Accepts a hex-encoded bincode-serialized Transaction.
/// Adds it to the mempool after validation.
pub fn send_transaction(ctx: &mut RpcContext, req: &RpcRequest) -> RpcResult {
    let tx_hex   = req.require_string(0, "tx_hex")?;
    let tx_bytes = hex::decode(&tx_hex)
        .map_err(|_| RpcError::invalid_tx("invalid hex encoding"))?;

    let tx: transaction::Transaction = bincode::deserialize(&tx_bytes)
        .map_err(|_| RpcError::invalid_tx("could not deserialize transaction"))?;

    // Structural validation.
    transaction::TransactionValidator::validate_structure(&tx)
        .map_err(|e| RpcError::invalid_tx(&e.to_string()))?;

    let tx_id = hex::encode(tx.tx_id().as_bytes());

    // Add to mempool.
    ctx.mempool.add(tx)
        .map_err(|e| RpcError::invalid_tx(&e.to_string()))?;

    Ok(serde_json::json!(tx_id))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_hash(hex_str: &str) -> Result<crypto::HashDigest, RpcError> {
    let bytes = hex::decode(hex_str)
        .map_err(|_| RpcError::invalid_hash("invalid hex"))?;
    bincode::deserialize(&bytes)
        .map_err(|_| RpcError::invalid_hash("invalid hash bytes"))
}
