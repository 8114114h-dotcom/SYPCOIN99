// handlers/miner.rs — Mining information handlers.

use block::difficulty_to_target;

use crate::error::{RpcError, RpcResult};
use crate::server::http::RpcContext;
use crate::types::requests::RpcRequest;
use crate::types::responses::MiningInfo;

/// getMiningInfo() → MiningInfo
pub fn get_mining_info(ctx: &mut RpcContext, _req: &RpcRequest) -> RpcResult {
    let height     = ctx.chain.height().as_u64();
    let difficulty = ctx.chain.current_difficulty();
    let best_hash  = hex::encode(ctx.chain.tip().hash().as_bytes());
    let target     = hex::encode(difficulty_to_target(difficulty));

    let info = MiningInfo { height, difficulty, best_hash, target };
    Ok(serde_json::to_value(info)
        .map_err(|e| RpcError::internal(&e.to_string()))?)
}

/// getBlockTemplate() → block template info for miners
///
/// Returns the data a miner needs to build and mine the next block.
pub fn get_block_template(ctx: &mut RpcContext, _req: &RpcRequest) -> RpcResult {
    let tip        = ctx.chain.tip();
    let height     = tip.height().next().as_u64();
    let parent     = hex::encode(tip.hash().as_bytes());
    let difficulty = ctx.chain.current_difficulty();
    let target     = hex::encode(difficulty_to_target(difficulty));

    // Get top transactions from mempool.
    let txs: Vec<_> = ctx.mempool
        .top_n(primitives::constants::MAX_TX_PER_BLOCK as usize)
        .iter()
        .map(|tx| hex::encode(tx.tx_id().as_bytes()))
        .collect();

    Ok(serde_json::json!({
        "height":     height,
        "parent":     parent,
        "difficulty": difficulty,
        "target":     target,
        "tx_ids":     txs,
        "tx_count":   txs.len(),
    }))
}
