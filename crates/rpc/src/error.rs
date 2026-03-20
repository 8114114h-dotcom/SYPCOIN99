// error.rs — RPC error codes (JSON-RPC 2.0 standard + custom).

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── Standard JSON-RPC 2.0 error codes ────────────────────────────────────────
pub const PARSE_ERROR:      i32 = -32700;
pub const INVALID_REQUEST:  i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS:   i32 = -32602;
pub const INTERNAL_ERROR:   i32 = -32603;

// ── Custom application error codes ───────────────────────────────────────────
pub const NOT_FOUND:        i32 = -32001;
pub const INVALID_ADDRESS:  i32 = -32002;
pub const INVALID_TX:       i32 = -32003;
pub const INVALID_HASH:     i32 = -32004;

/// A JSON-RPC error object returned inside RpcResponse.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RpcError {
    pub code:    i32,
    pub message: String,
}

impl RpcError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        RpcError { code, message: message.into() }
    }

    pub fn parse_error()              -> Self { Self::new(PARSE_ERROR,      "Parse error") }
    pub fn invalid_request()          -> Self { Self::new(INVALID_REQUEST,  "Invalid request") }
    pub fn method_not_found(m: &str)  -> Self { Self::new(METHOD_NOT_FOUND, format!("Method not found: {}", m)) }
    pub fn invalid_params(msg: &str)  -> Self { Self::new(INVALID_PARAMS,   format!("Invalid params: {}", msg)) }
    pub fn internal(msg: &str)        -> Self { Self::new(INTERNAL_ERROR,   format!("Internal error: {}", msg)) }
    pub fn not_found(what: &str)      -> Self { Self::new(NOT_FOUND,        format!("Not found: {}", what)) }
    pub fn invalid_address(msg: &str) -> Self { Self::new(INVALID_ADDRESS,  format!("Invalid address: {}", msg)) }
    pub fn invalid_tx(msg: &str)      -> Self { Self::new(INVALID_TX,       format!("Invalid transaction: {}", msg)) }
    pub fn invalid_hash(msg: &str)    -> Self { Self::new(INVALID_HASH,     format!("Invalid hash: {}", msg)) }
}

/// Internal error type used within handlers before converting to RpcError.
#[derive(Debug, Error)]
pub enum RpcHandlerError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("invalid address: {0}")]
    InvalidAddress(String),

    #[error("invalid transaction: {0}")]
    InvalidTx(String),

    #[error("invalid hash: {0}")]
    InvalidHash(String),

    #[error("invalid params: {0}")]
    InvalidParams(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl From<RpcHandlerError> for RpcError {
    fn from(e: RpcHandlerError) -> Self {
        match e {
            RpcHandlerError::NotFound(m)       => RpcError::not_found(&m),
            RpcHandlerError::InvalidAddress(m) => RpcError::invalid_address(&m),
            RpcHandlerError::InvalidTx(m)      => RpcError::invalid_tx(&m),
            RpcHandlerError::InvalidHash(m)    => RpcError::invalid_hash(&m),
            RpcHandlerError::InvalidParams(m)  => RpcError::invalid_params(&m),
            RpcHandlerError::Internal(m)       => RpcError::internal(&m),
        }
    }
}

pub type RpcResult = Result<serde_json::Value, RpcError>;
