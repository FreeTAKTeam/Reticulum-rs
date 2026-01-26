pub mod codec;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct RpcRequest {
    pub id: u64,
    pub method: String,
    pub params: Option<JsonValue>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct RpcResponse {
    pub id: u64,
    pub result: Option<JsonValue>,
    pub error: Option<RpcError>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct RpcError {
    pub code: String,
    pub message: String,
}
