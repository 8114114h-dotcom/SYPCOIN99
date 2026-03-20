// types/requests.rs — JSON-RPC 2.0 request and response envelope types.

use serde::{Deserialize, Serialize};
use crate::error::RpcError;

/// A JSON-RPC 2.0 request.
#[derive(Clone, Debug, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub method:  String,
    #[serde(default)]
    pub params:  serde_json::Value,
    pub id:      serde_json::Value,
}

impl RpcRequest {
    /// Validate the envelope (must be jsonrpc="2.0").
    pub fn validate(&self) -> Result<(), RpcError> {
        if self.jsonrpc != "2.0" {
            return Err(RpcError::invalid_request());
        }
        Ok(())
    }

    /// Extract a positional param by index.
    pub fn param_at(&self, idx: usize) -> Option<&serde_json::Value> {
        self.params.as_array()?.get(idx)
    }

    /// Extract a named param by key.
    pub fn param_named(&self, key: &str) -> Option<&serde_json::Value> {
        self.params.as_object()?.get(key)
    }

    /// Extract a required string param at position `idx`.
    pub fn require_string(&self, idx: usize, name: &str) -> Result<String, RpcError> {
        self.param_at(idx)
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned())
            .ok_or_else(|| RpcError::invalid_params(&format!("missing string param '{}'", name)))
    }

    /// Extract a required u64 param at position `idx`.
    pub fn require_u64(&self, idx: usize, name: &str) -> Result<u64, RpcError> {
        self.param_at(idx)
            .and_then(|v| v.as_u64())
            .ok_or_else(|| RpcError::invalid_params(&format!("missing u64 param '{}'", name)))
    }
}

/// A JSON-RPC 2.0 response.
#[derive(Clone, Debug, Serialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result:  Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error:   Option<RpcError>,
    pub id:      serde_json::Value,
}

impl RpcResponse {
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        RpcResponse { jsonrpc: "2.0".into(), result: Some(result), error: None, id }
    }

    pub fn error(id: serde_json::Value, error: RpcError) -> Self {
        RpcResponse { jsonrpc: "2.0".into(), result: None, error: Some(error), id }
    }
}
