pub mod codec;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};

use crate::storage::messages::MessagesStore;

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

pub struct RpcDaemon {
    store: MessagesStore,
    identity_hash: String,
}

impl RpcDaemon {
    pub fn test_instance() -> Self {
        let store = MessagesStore::in_memory().expect("in-memory store");
        Self {
            store,
            identity_hash: "test-identity".into(),
        }
    }

    pub fn handle_rpc(&self, request: RpcRequest) -> Result<RpcResponse, std::io::Error> {
        match request.method.as_str() {
            "status" => Ok(RpcResponse {
                id: request.id,
                result: Some(json!({
                    "identity_hash": self.identity_hash,
                    "running": true
                })),
                error: None,
            }),
            "list_messages" => {
                let items = self
                    .store
                    .list_messages(100, None)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "messages": items })),
                    error: None,
                })
            }
            _ => Ok(RpcResponse {
                id: request.id,
                result: None,
                error: Some(RpcError {
                    code: "NOT_IMPLEMENTED".into(),
                    message: "method not implemented".into(),
                }),
            }),
        }
    }
}
