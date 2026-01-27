pub mod codec;
pub mod http;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};

use crate::storage::messages::MessagesStore;
use std::collections::VecDeque;
use std::sync::Mutex;
use tokio::sync::broadcast;
use tokio::time::Duration;

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
    events: broadcast::Sender<RpcEvent>,
    event_queue: Mutex<VecDeque<RpcEvent>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct RpcEvent {
    pub event_type: String,
    pub payload: JsonValue,
}

#[derive(Debug, Deserialize)]
struct SendMessageParams {
    id: String,
    source: String,
    destination: String,
    content: String,
}

impl RpcDaemon {
    pub fn with_store(store: MessagesStore, identity_hash: String) -> Self {
        let (events, _rx) = broadcast::channel(64);
        Self {
            store,
            identity_hash,
            events,
            event_queue: Mutex::new(VecDeque::new()),
        }
    }

    pub fn test_instance() -> Self {
        let store = MessagesStore::in_memory().expect("in-memory store");
        Self::with_store(store, "test-identity".into())
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
            "list_peers" => Ok(RpcResponse {
                id: request.id,
                result: Some(json!({ "peers": [] })),
                error: None,
            }),
            "send_message" => {
                let params = request
                    .params
                    .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params"))?;
                let parsed: SendMessageParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|value| value.as_secs() as i64)
                    .unwrap_or(0);
                let record = crate::storage::messages::MessageRecord {
                    id: parsed.id.clone(),
                    source: parsed.source,
                    destination: parsed.destination,
                    content: parsed.content,
                    timestamp,
                    direction: "out".into(),
                };
                self.store
                    .insert_message(&record)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "message_id": record.id })),
                    error: None,
                })
            }
            "receive_message" => {
                let params = request
                    .params
                    .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params"))?;
                let parsed: SendMessageParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|value| value.as_secs() as i64)
                    .unwrap_or(0);
                let record = crate::storage::messages::MessageRecord {
                    id: parsed.id.clone(),
                    source: parsed.source,
                    destination: parsed.destination,
                    content: parsed.content,
                    timestamp,
                    direction: "in".into(),
                };
                self.store
                    .insert_message(&record)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
                let event = RpcEvent {
                    event_type: "inbound".into(),
                    payload: json!({ "message": record }),
                };
                self.push_event(event.clone());
                let _ = self.events.send(event);
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "message_id": parsed.id })),
                    error: None,
                })
            }
            "announce_now" => {
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|value| value.as_secs() as i64)
                    .unwrap_or(0);
                let event = RpcEvent {
                    event_type: "announce_sent".into(),
                    payload: json!({ "timestamp": timestamp }),
                };
                self.push_event(event.clone());
                let _ = self.events.send(event);
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "announce_id": request.id })),
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

    pub fn handle_framed_request(&self, bytes: &[u8]) -> Result<Vec<u8>, std::io::Error> {
        let request: RpcRequest = codec::decode_frame(bytes)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
        let response = self.handle_rpc(request)?;
        codec::encode_frame(&response)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<RpcEvent> {
        self.events.subscribe()
    }

    pub fn take_event(&self) -> Option<RpcEvent> {
        let mut guard = self
            .event_queue
            .lock()
            .expect("event_queue mutex poisoned");
        guard.pop_front()
    }

    pub fn push_event(&self, event: RpcEvent) {
        let mut guard = self
            .event_queue
            .lock()
            .expect("event_queue mutex poisoned");
        if guard.len() >= 32 {
            guard.pop_front();
        }
        guard.push_back(event);
    }

    pub fn schedule_announce_for_test(&self, id: u64) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|value| value.as_secs() as i64)
            .unwrap_or(0);
        let event = RpcEvent {
            event_type: "announce_sent".into(),
            payload: json!({ "timestamp": timestamp, "announce_id": id }),
        };
        self.push_event(event.clone());
        let _ = self.events.send(event);
    }

    pub fn start_announce_scheduler(self: std::sync::Arc<Self>, interval_secs: u64) -> tokio::task::JoinHandle<()> {
        tokio::task::spawn_local(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
            interval.tick().await;
            loop {
                interval.tick().await;
                let id = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|value| value.as_secs() as u64)
                    .unwrap_or(0);
                self.schedule_announce_for_test(id);
            }
        })
    }

    pub fn inject_inbound_test_message(&self, content: &str) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|value| value.as_secs() as i64)
            .unwrap_or(0);
        let record = crate::storage::messages::MessageRecord {
            id: format!("test-{}", timestamp),
            source: "test-peer".into(),
            destination: "local".into(),
            content: content.into(),
            timestamp,
            direction: "in".into(),
        };
        let _ = self.store.insert_message(&record);
        let event = RpcEvent {
            event_type: "inbound".into(),
            payload: json!({ "message": record }),
        };
        self.push_event(event.clone());
        let _ = self.events.send(event);
    }
}

pub fn handle_framed_request(
    daemon: &RpcDaemon,
    bytes: &[u8],
) -> Result<Vec<u8>, std::io::Error> {
    daemon.handle_framed_request(bytes)
}
