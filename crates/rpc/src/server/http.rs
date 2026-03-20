// server/http.rs — JSON-RPC dispatcher and context.
//
// RpcContext holds references to all shared state.
// The actual HTTP server (using hyper/axum) lives in 11_node.
// This module provides the dispatch() function that maps method names
// to handlers — it is transport-agnostic.

use consensus::Blockchain;
use state::WorldState;
use storage::Storage;
use transaction::Mempool;

use crate::error::{RpcError, RpcResult};
use crate::handlers::{account, chain, miner, tx};
use crate::types::requests::{RpcRequest, RpcResponse};

/// Shared context passed to every RPC handler.
///
/// All fields are cloned from Arc<RwLock<>> in the node layer.
/// For this crate's dispatch logic, we accept owned values
/// (the node layer wraps them in locks before calling dispatch).
pub struct RpcContext {
    pub storage: Storage,
    pub state:   WorldState,
    pub mempool: Mempool,
    pub chain:   Blockchain,
}

/// Dispatch a JSON-RPC request to the appropriate handler.
///
/// This is the single entry point from the transport layer.
pub fn dispatch(ctx: &mut RpcContext, req: RpcRequest) -> RpcResponse {
    // Validate JSON-RPC envelope.
    if let Err(e) = req.validate() {
        return RpcResponse::error(req.id.clone(), e);
    }

    let result: RpcResult = match req.method.as_str() {
        // Chain
        "getBlockHeight"    => chain::get_block_height(ctx, &req),
        "getBlock"          => chain::get_block(ctx, &req),
        "getBlockByHeight"  => chain::get_block_by_height(ctx, &req),
        "getBlockHash"      => chain::get_block_hash(ctx, &req),

        // Account
        "getBalance"        => account::get_balance(ctx, &req),
        "getNonce"          => account::get_nonce(ctx, &req),

        // Transaction
        "getTransaction"    => tx::get_transaction(ctx, &req),
        "sendTransaction"   => tx::send_transaction(ctx, &req),

        // Miner
        "getMiningInfo"     => miner::get_mining_info(ctx, &req),
        "getBlockTemplate"  => miner::get_block_template(ctx, &req),

        unknown => Err(RpcError::method_not_found(unknown)),
    };

    match result {
        Ok(value) => RpcResponse::success(req.id, value),
        Err(err)  => RpcResponse::error(req.id, err),
    }
}

/// Parse a JSON string into an RpcRequest.
pub fn parse_request(body: &str) -> Result<RpcRequest, RpcError> {
    serde_json::from_str(body).map_err(|_| RpcError::parse_error())
}

/// Serialize an RpcResponse to a JSON string.
pub fn serialize_response(resp: &RpcResponse) -> String {
    serde_json::to_string(resp).unwrap_or_else(|_| {
        r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Response serialization failed"},"id":null}"#
            .to_owned()
    })
}
