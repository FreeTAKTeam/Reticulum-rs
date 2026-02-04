pub mod codec;
pub mod http;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};

use crate::storage::messages::{MessageRecord, MessagesStore};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
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
    peers: Mutex<HashMap<String, PeerRecord>>,
    outbound_bridge: Option<Arc<dyn OutboundBridge>>,
    announce_bridge: Option<Arc<dyn AnnounceBridge>>,
}

pub trait OutboundBridge: Send + Sync {
    fn deliver(&self, record: &MessageRecord) -> Result<(), std::io::Error>;
}

pub trait AnnounceBridge: Send + Sync {
    fn announce_now(&self) -> Result<(), std::io::Error>;
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct RpcEvent {
    pub event_type: String,
    pub payload: JsonValue,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PeerRecord {
    pub peer: String,
    pub last_seen: i64,
}

#[derive(Debug, Deserialize)]
struct SendMessageParams {
    id: String,
    source: String,
    destination: String,
    #[serde(default)]
    title: String,
    content: String,
    fields: Option<JsonValue>,
}

#[derive(Debug, Deserialize)]
struct RecordReceiptParams {
    message_id: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct AnnounceReceivedParams {
    peer: String,
    timestamp: Option<i64>,
}

impl RpcDaemon {
    pub fn with_store(store: MessagesStore, identity_hash: String) -> Self {
        let (events, _rx) = broadcast::channel(64);
        Self {
            store,
            identity_hash,
            events,
            event_queue: Mutex::new(VecDeque::new()),
            peers: Mutex::new(HashMap::new()),
            outbound_bridge: None,
            announce_bridge: None,
        }
    }

    pub fn with_store_and_bridge(
        store: MessagesStore,
        identity_hash: String,
        outbound_bridge: Arc<dyn OutboundBridge>,
    ) -> Self {
        let (events, _rx) = broadcast::channel(64);
        Self {
            store,
            identity_hash,
            events,
            event_queue: Mutex::new(VecDeque::new()),
            peers: Mutex::new(HashMap::new()),
            outbound_bridge: Some(outbound_bridge),
            announce_bridge: None,
        }
    }

    pub fn with_store_and_bridges(
        store: MessagesStore,
        identity_hash: String,
        outbound_bridge: Option<Arc<dyn OutboundBridge>>,
        announce_bridge: Option<Arc<dyn AnnounceBridge>>,
    ) -> Self {
        let (events, _rx) = broadcast::channel(64);
        Self {
            store,
            identity_hash,
            events,
            event_queue: Mutex::new(VecDeque::new()),
            peers: Mutex::new(HashMap::new()),
            outbound_bridge,
            announce_bridge,
        }
    }

    pub fn test_instance() -> Self {
        let store = MessagesStore::in_memory().expect("in-memory store");
        Self::with_store(store, "test-identity".into())
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn test_instance_with_identity(identity: impl Into<String>) -> Self {
        let store = MessagesStore::in_memory().expect("in-memory store");
        Self::with_store(store, identity.into())
    }

    fn store_inbound_record(&self, record: MessageRecord) -> Result<(), std::io::Error> {
        self.store
            .insert_message(&record)
            .map_err(std::io::Error::other)?;
        let event = RpcEvent {
            event_type: "inbound".into(),
            payload: json!({ "message": record }),
        };
        self.push_event(event.clone());
        let _ = self.events.send(event);
        Ok(())
    }

    pub fn accept_inbound(&self, record: MessageRecord) -> Result<(), std::io::Error> {
        self.store_inbound_record(record)
    }

    pub fn accept_announce(&self, peer: String, timestamp: i64) -> Result<(), std::io::Error> {
        let record = PeerRecord { peer, last_seen: timestamp };
        {
            let mut guard = self.peers.lock().expect("peers mutex poisoned");
            guard.insert(record.peer.clone(), record.clone());
        }
        let event = RpcEvent {
            event_type: "announce_received".into(),
            payload: json!({ "peer": record.peer, "timestamp": record.last_seen }),
        };
        self.push_event(event.clone());
        let _ = self.events.send(event);
        Ok(())
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn accept_inbound_for_test(
        &self,
        record: MessageRecord,
    ) -> Result<(), std::io::Error> {
        self.store_inbound_record(record)
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
                    .map_err(std::io::Error::other)?;
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "messages": items })),
                    error: None,
                })
            }
            "list_peers" => {
                let peers = self
                    .peers
                    .lock()
                    .expect("peers mutex poisoned")
                    .values()
                    .cloned()
                    .collect::<Vec<_>>();
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "peers": peers })),
                    error: None,
                })
            }
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
                let record = MessageRecord {
                    id: parsed.id.clone(),
                    source: parsed.source,
                    destination: parsed.destination,
                    title: parsed.title,
                    content: parsed.content,
                    timestamp,
                    direction: "out".into(),
                    fields: parsed.fields,
                    receipt_status: None,
                };
                self.store
                    .insert_message(&record)
                    .map_err(std::io::Error::other)?;
                if let Some(bridge) = &self.outbound_bridge {
                    let _ = bridge.deliver(&record);
                } else {
                    let _delivered = crate::transport::test_bridge::deliver_outbound(&record);
                }
                let event = RpcEvent {
                    event_type: "outbound".into(),
                    payload: json!({ "message": record }),
                };
                self.push_event(event.clone());
                let _ = self.events.send(event);
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
                let record = MessageRecord {
                    id: parsed.id.clone(),
                    source: parsed.source,
                    destination: parsed.destination,
                    title: parsed.title,
                    content: parsed.content,
                    timestamp,
                    direction: "in".into(),
                    fields: parsed.fields,
                    receipt_status: None,
                };
                self.store_inbound_record(record)?;
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "message_id": parsed.id })),
                    error: None,
                })
            }
            "record_receipt" => {
                let params = request
                    .params
                    .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params"))?;
                let parsed: RecordReceiptParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                self.store
                    .update_receipt_status(&parsed.message_id, &parsed.status)
                    .map_err(std::io::Error::other)?;
                let event = RpcEvent {
                    event_type: "receipt".into(),
                    payload: json!({ "message_id": parsed.message_id, "status": parsed.status }),
                };
                self.push_event(event.clone());
                let _ = self.events.send(event);
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "message_id": parsed.message_id, "status": parsed.status })),
                    error: None,
                })
            }
            "announce_now" => {
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|value| value.as_secs() as i64)
                    .unwrap_or(0);
                if let Some(bridge) = &self.announce_bridge {
                    let _ = bridge.announce_now();
                }
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
            "announce_received" => {
                let params = request
                    .params
                    .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params"))?;
                let parsed: AnnounceReceivedParams = serde_json::from_value(params)
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
                let timestamp = parsed.timestamp.unwrap_or_else(|| {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|value| value.as_secs() as i64)
                        .unwrap_or(0)
                });
                let record = PeerRecord {
                    peer: parsed.peer,
                    last_seen: timestamp,
                };
                {
                    let mut guard = self.peers.lock().expect("peers mutex poisoned");
                    guard.insert(record.peer.clone(), record.clone());
                }
                let event = RpcEvent {
                    event_type: "announce_received".into(),
                    payload: json!({ "peer": record.peer, "timestamp": record.last_seen }),
                };
                self.push_event(event.clone());
                let _ = self.events.send(event);
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "peer": record })),
                    error: None,
                })
            }
            "clear_messages" => {
                self.store
                    .clear_messages()
                    .map_err(std::io::Error::other)?;
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "cleared": "messages" })),
                    error: None,
                })
            }
            "clear_resources" => Ok(RpcResponse {
                id: request.id,
                result: Some(json!({ "cleared": "resources" })),
                error: None,
            }),
            "clear_peers" => {
                {
                    let mut guard = self.peers.lock().expect("peers mutex poisoned");
                    guard.clear();
                }
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "cleared": "peers" })),
                    error: None,
                })
            }
            "clear_all" => {
                self.store
                    .clear_messages()
                    .map_err(std::io::Error::other)?;
                {
                    let mut guard = self.peers.lock().expect("peers mutex poisoned");
                    guard.clear();
                }
                Ok(RpcResponse {
                    id: request.id,
                    result: Some(json!({ "cleared": "all" })),
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
            .map_err(std::io::Error::other)
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

    pub fn start_announce_scheduler(self: std::rc::Rc<Self>, interval_secs: u64) -> tokio::task::JoinHandle<()> {
        tokio::task::spawn_local(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
            interval.tick().await;
            loop {
                interval.tick().await;
                let id = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|value| value.as_secs())
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
            title: "".into(),
            content: content.into(),
            timestamp,
            direction: "in".into(),
            fields: None,
            receipt_status: None,
        };
        let _ = self.store.insert_message(&record);
        let event = RpcEvent {
            event_type: "inbound".into(),
            payload: json!({ "message": record }),
        };
        self.push_event(event.clone());
        let _ = self.events.send(event);
    }

    pub fn emit_link_event_for_test(&self) {
        let event = RpcEvent {
            event_type: "link_activated".into(),
            payload: json!({ "link_id": "test-link" }),
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
