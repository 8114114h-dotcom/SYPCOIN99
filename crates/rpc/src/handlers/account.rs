// handlers/account.rs — Account query handlers.

use crypto::Address;
use primitives::micro_to_display;

use crate::error::{RpcError, RpcResult};
use crate::server::http::RpcContext;
use crate::types::requests::RpcRequest;

/// getBalance(address: String) → String (decimal display)
pub fn get_balance(ctx: &mut RpcContext, req: &RpcRequest) -> RpcResult {
    let addr = parse_address(req, 0)?;
    let balance = ctx.state.get_balance(&addr);
    Ok(serde_json::json!(micro_to_display(balance.as_micro())))
}

/// getNonce(address: String) → u64
pub fn get_nonce(ctx: &mut RpcContext, req: &RpcRequest) -> RpcResult {
    let addr  = parse_address(req, 0)?;
    let nonce = ctx.state.get_nonce(&addr);
    Ok(serde_json::json!(nonce.as_u64()))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_address(req: &RpcRequest, idx: usize) -> Result<Address, RpcError> {
    let addr_str = req.require_string(idx, "address")?;
    Address::from_checksum_hex(&addr_str)
        .map_err(|_| RpcError::invalid_address(&format!("'{}' is not valid", addr_str)))
}
