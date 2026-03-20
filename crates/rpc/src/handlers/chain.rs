// handlers/chain.rs — Chain query handlers.

use primitives::BlockHeight;

use crate::error::{RpcError, RpcResult};
use crate::server::http::RpcContext;
use crate::types::requests::RpcRequest;
use crate::types::responses::BlockResponse;

/// getBlockHeight() → u64
pub fn get_block_height(ctx: &mut RpcContext, _req: &RpcRequest) -> RpcResult {
    let height = ctx.storage.chain_height()
        .map_err(|e| RpcError::internal(&e.to_string()))?;
    Ok(serde_json::json!(height))
}

/// getBlock(hash: String) → BlockResponse
pub fn get_block(ctx: &mut RpcContext, req: &RpcRequest) -> RpcResult {
    let hash_hex  = req.require_string(0, "hash")?;
    let hash_bytes = hex::decode(&hash_hex)
        .map_err(|_| RpcError::invalid_hash("invalid hex"))?;
    if hash_bytes.len() != 32 {
        return Err(RpcError::invalid_hash("hash must be 32 bytes"));
    }
    let hash = bytes_to_hash(&hash_bytes)?;

    match ctx.storage.get_block(&hash)
        .map_err(|e| RpcError::internal(&e.to_string()))?
    {
        None        => Err(RpcError::not_found("block")),
        Some(block) => Ok(serde_json::to_value(BlockResponse::from_block(&block))
            .map_err(|e| RpcError::internal(&e.to_string()))?),
    }
}

/// getBlockByHeight(height: u64) → BlockResponse
pub fn get_block_by_height(ctx: &mut RpcContext, req: &RpcRequest) -> RpcResult {
    let height = req.require_u64(0, "height")?;

    match ctx.storage.get_block_at(BlockHeight::new(height))
        .map_err(|e| RpcError::internal(&e.to_string()))?
    {
        None        => Err(RpcError::not_found("block")),
        Some(block) => Ok(serde_json::to_value(BlockResponse::from_block(&block))
            .map_err(|e| RpcError::internal(&e.to_string()))?),
    }
}

/// getBlockHash(height: u64) → String
pub fn get_block_hash(ctx: &mut RpcContext, req: &RpcRequest) -> RpcResult {
    let height = req.require_u64(0, "height")?;

    match ctx.storage.get_block_at(BlockHeight::new(height))
        .map_err(|e| RpcError::internal(&e.to_string()))?
    {
        None        => Err(RpcError::not_found("block")),
        Some(block) => Ok(serde_json::json!(hex::encode(block.hash().as_bytes() as &[u8]))),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn bytes_to_hash(bytes: &[u8]) -> Result<crypto::HashDigest, RpcError> {
    if bytes.len() != 32 {
        return Err(RpcError::invalid_hash("expected 32 bytes"));
    }
    bincode::deserialize(bytes)
        .map_err(|_| RpcError::invalid_hash("could not decode hash"))
}
