// lib.rs — Public API for the rpc crate.
//
//   use rpc::{dispatch, parse_request, serialize_response, RpcContext};
//   use rpc::{RpcRequest, RpcResponse, RpcError};

mod error;
mod middleware;

mod types {
    pub(crate) mod requests;
    pub(crate) mod responses;
}

mod handlers {
    pub(crate) mod account;
    pub(crate) mod chain;
    pub(crate) mod miner;
    pub(crate) mod tx;
}

mod server {
    pub(crate) mod http;
}

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use error::{RpcError, RpcResult, RpcHandlerError};
pub use error::{PARSE_ERROR, INVALID_REQUEST, METHOD_NOT_FOUND,
                INVALID_PARAMS, INTERNAL_ERROR, NOT_FOUND,
                INVALID_ADDRESS, INVALID_TX, INVALID_HASH};
pub use types::requests::{RpcRequest, RpcResponse};
pub use server::http::{RpcContext, dispatch, parse_request, serialize_response};
pub use middleware::RpcRateLimiter;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use consensus::Blockchain;
    use crypto::{Address, KeyPair, sha256};
    use primitives::{Amount, BlockHeight, Nonce, Timestamp};
    use primitives::constants::MIN_TX_FEE_MICRO;
    use state::WorldState;
    use storage::Storage;
    use transaction::{Mempool, TransactionBuilder};
    use block::{Block, BlockBuilder};

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn make_address() -> Address {
        Address::from_public_key(KeyPair::generate().unwrap().public_key())
    }

    fn zero_hash() -> crypto::HashDigest { sha256(b"zero") }

    fn make_genesis() -> Block {
        BlockBuilder::new()
            .height(BlockHeight::new(0))
            .parent_hash(zero_hash())
            .state_root(zero_hash())
            .miner(make_address())
            .difficulty(1)
            .timestamp(Timestamp::from_millis(1_700_000_000_000))
            .build()
            .unwrap()
    }

    fn make_ctx() -> RpcContext {
        let genesis = make_genesis();
        let mut storage = Storage::open_in_memory();
        storage.save_block(&genesis).unwrap();

        let mut state = WorldState::new();
        state.commit(BlockHeight::new(0));

        let chain   = Blockchain::new(genesis, 1).unwrap();
        let mempool = Mempool::with_defaults();

        RpcContext { storage, state, mempool, chain }
    }

    fn rpc_req(method: &str, params: serde_json::Value) -> RpcRequest {
        RpcRequest {
            jsonrpc: "2.0".into(),
            method:  method.into(),
            params,
            id:      serde_json::json!(1),
        }
    }

    // ── parse_request / serialize_response ────────────────────────────────────

    #[test]
    fn test_parse_valid_request() {
        let json = r#"{"jsonrpc":"2.0","method":"getBlockHeight","params":[],"id":1}"#;
        let req  = parse_request(json).unwrap();
        assert_eq!(req.method, "getBlockHeight");
    }

    #[test]
    fn test_parse_invalid_json() {
        let result = parse_request("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_serialize_response() {
        let resp = RpcResponse::success(
            serde_json::json!(1),
            serde_json::json!(42),
        );
        let json = serialize_response(&resp);
        assert!(json.contains("\"result\":42"));
    }

    // ── dispatch ──────────────────────────────────────────────────────────────

    #[test]
    fn test_dispatch_unknown_method() {
        let mut ctx  = make_ctx();
        let req  = rpc_req("unknownMethod", serde_json::json!([]));
        let resp = dispatch(&mut ctx, req);
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, METHOD_NOT_FOUND);
    }

    #[test]
    fn test_dispatch_invalid_jsonrpc_version() {
        let ctx = make_ctx();
        let req = RpcRequest {
            jsonrpc: "1.0".into(), // wrong version
            method:  "getBlockHeight".into(),
            params:  serde_json::json!([]),
            id:      serde_json::json!(1),
        };
        let resp = dispatch(&mut ctx, req);
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, INVALID_REQUEST);
    }

    // ── Chain handlers ────────────────────────────────────────────────────────

    #[test]
    fn test_get_block_height() {
        let mut ctx  = make_ctx();
        let req  = rpc_req("getBlockHeight", serde_json::json!([]));
        let resp = dispatch(&mut ctx, req);
        assert!(resp.error.is_none());
        assert_eq!(resp.result.unwrap(), serde_json::json!(0));
    }

    #[test]
    fn test_get_block_by_height_genesis() {
        let mut ctx  = make_ctx();
        let req  = rpc_req("getBlockByHeight", serde_json::json!([0]));
        let resp = dispatch(&mut ctx, req);
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["height"], 0);
    }

    #[test]
    fn test_get_block_by_height_not_found() {
        let mut ctx  = make_ctx();
        let req  = rpc_req("getBlockByHeight", serde_json::json!([999]));
        let resp = dispatch(&mut ctx, req);
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, NOT_FOUND);
    }

    #[test]
    fn test_get_block_hash_genesis() {
        let mut ctx  = make_ctx();
        let req  = rpc_req("getBlockHash", serde_json::json!([0]));
        let resp = dispatch(&mut ctx, req);
        assert!(resp.error.is_none());
        let hash = resp.result.unwrap();
        assert!(hash.as_str().unwrap().len() == 64);
    }

    // ── Account handlers ──────────────────────────────────────────────────────

    #[test]
    fn test_get_balance_unknown_address() {
        let addr = make_address();
        let mut ctx  = make_ctx();
        let req  = rpc_req("getBalance", serde_json::json!([addr.to_checksum_hex()]));
        let resp = dispatch(&mut ctx, req);
        // Unknown address returns 0, not error.
        assert!(resp.error.is_none());
        assert_eq!(resp.result.unwrap(), "0.000000");
    }

    #[test]
    fn test_get_balance_invalid_address() {
        let mut ctx  = make_ctx();
        let req  = rpc_req("getBalance", serde_json::json!(["notanaddress"]));
        let resp = dispatch(&mut ctx, req);
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, INVALID_ADDRESS);
    }

    #[test]
    fn test_get_nonce_unknown_address() {
        let addr = make_address();
        let mut ctx  = make_ctx();
        let req  = rpc_req("getNonce", serde_json::json!([addr.to_checksum_hex()]));
        let resp = dispatch(&mut ctx, req);
        assert!(resp.error.is_none());
        assert_eq!(resp.result.unwrap(), 0);
    }

    // ── Miner handlers ────────────────────────────────────────────────────────

    #[test]
    fn test_get_mining_info() {
        let mut ctx  = make_ctx();
        let req  = rpc_req("getMiningInfo", serde_json::json!([]));
        let resp = dispatch(&mut ctx, req);
        assert!(resp.error.is_none());
        let info = resp.result.unwrap();
        assert_eq!(info["height"], 0);
        assert!(info["difficulty"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn test_get_block_template() {
        let mut ctx  = make_ctx();
        let req  = rpc_req("getBlockTemplate", serde_json::json!([]));
        let resp = dispatch(&mut ctx, req);
        assert!(resp.error.is_none());
        let tmpl = resp.result.unwrap();
        assert_eq!(tmpl["height"], 1);
    }

    // ── Transaction handlers ──────────────────────────────────────────────────

    #[test]
    fn test_get_transaction_not_found() {
        let ctx     = make_ctx();
        let fake_id = hex::encode(sha256(b"fake").as_bytes());
        let req     = rpc_req("getTransaction", serde_json::json!([fake_id]));
        let resp    = dispatch(&ctx, req);
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, NOT_FOUND);
    }

    #[test]
    fn test_send_transaction_invalid_hex() {
        let mut ctx  = make_ctx();
        let req  = rpc_req("sendTransaction", serde_json::json!(["notvalidhex"]));
        let resp = dispatch(&mut ctx, req);
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, INVALID_TX);
    }

    #[test]
    fn test_send_transaction_valid() {
        let kp    = KeyPair::generate().unwrap();
        let tx = TransactionBuilder::new()
            .from_keypair(kp)
            .to(make_address())
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(1))
            .build()
            .unwrap();

        let tx_bytes = bincode::serialize(&tx).unwrap();
        let tx_hex   = hex::encode(&tx_bytes);

        let mut ctx  = make_ctx();
        let req  = rpc_req("sendTransaction", serde_json::json!([tx_hex]));
        let resp = dispatch(&mut ctx, req);
        // Should succeed (tx added to mempool).
        assert!(resp.error.is_none());
        let tx_id = resp.result.unwrap();
        assert_eq!(tx_id.as_str().unwrap().len(), 64);
    }

    // ── RateLimiter ───────────────────────────────────────────────────────────

    #[test]
    fn test_rate_limiter_allows_under_limit() {
        let mut rl = RpcRateLimiter::new();
        for _ in 0..50 {
            assert!(rl.check("127.0.0.1"));
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let mut rl = RpcRateLimiter::new();
        for _ in 0..50 { rl.check("10.0.0.1"); }
        assert!(!rl.check("10.0.0.1"));
    }
}
