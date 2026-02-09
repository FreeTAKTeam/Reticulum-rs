use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use clap::Parser;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::LocalSet;

use reticulum::destination::{DestinationName, SingleInputDestination};
use reticulum::hash::AddressHash;
use reticulum::identity::{Identity, PrivateIdentity};
use reticulum::iface::tcp_client::TcpClient;
use reticulum::iface::tcp_server::TcpServer;
use reticulum::rpc::{http, AnnounceBridge, InterfaceRecord, OutboundBridge, RpcDaemon};
use reticulum::storage::messages::MessagesStore;
use reticulum::transport::{Transport, TransportConfig};
use tokio::sync::mpsc::unbounded_channel;

use reticulum_daemon::config::DaemonConfig;
use reticulum_daemon::direct_delivery::send_via_link;
use reticulum_daemon::identity_store::load_or_create_identity;
use reticulum_daemon::inbound_delivery::decode_inbound_payload;
use reticulum_daemon::lxmf_bridge::build_wire_message;
use reticulum_daemon::announce_names::{normalize_display_name, parse_peer_name_from_app_data};
use reticulum_daemon::receipt_bridge::{
    handle_receipt_event, track_receipt_mapping, ReceiptBridge,
};

#[derive(Parser, Debug)]
#[command(name = "reticulumd")]
struct Args {
    #[arg(long, default_value = "127.0.0.1:4243")]
    rpc: String,
    #[arg(long, default_value = "reticulum.db")]
    db: PathBuf,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    identity: Option<PathBuf>,
    #[arg(long, default_value_t = 0)]
    announce_interval_secs: u64,
    #[arg(long)]
    transport: Option<String>,
}

struct TransportBridge {
    transport: Arc<Transport>,
    signer: PrivateIdentity,
    announce_destination: Arc<tokio::sync::Mutex<SingleInputDestination>>,
    announce_app_data: Option<Vec<u8>>,
    peer_crypto: Arc<std::sync::Mutex<HashMap<String, PeerCrypto>>>,
    receipt_map: Arc<std::sync::Mutex<HashMap<String, String>>>,
}

#[derive(Clone, Copy)]
struct PeerCrypto {
    identity: Identity,
}

impl TransportBridge {
    fn new(
        transport: Arc<Transport>,
        signer: PrivateIdentity,
        announce_destination: Arc<tokio::sync::Mutex<SingleInputDestination>>,
        announce_app_data: Option<Vec<u8>>,
        peer_crypto: Arc<std::sync::Mutex<HashMap<String, PeerCrypto>>>,
        receipt_map: Arc<std::sync::Mutex<HashMap<String, String>>>,
    ) -> Self {
        Self {
            transport,
            signer,
            announce_destination,
            announce_app_data,
            peer_crypto,
            receipt_map,
        }
    }
}

impl OutboundBridge for TransportBridge {
    fn deliver(
        &self,
        record: &reticulum::storage::messages::MessageRecord,
    ) -> Result<(), std::io::Error> {
        let destination = parse_destination_hex(&record.destination);
        let Some(destination) = destination else {
            return Ok(());
        };
        let peer_info = self
            .peer_crypto
            .lock()
            .expect("peer map")
            .get(&record.destination)
            .copied();
        let Some(peer_info) = peer_info else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "peer not announced",
            ));
        };

        let source_hash = self.signer.address_hash();
        let mut source = [0u8; 16];
        source.copy_from_slice(source_hash.as_slice());

        let wire = build_wire_message(
            source,
            destination,
            &record.title,
            &record.content,
            record.fields.clone(),
            &self.signer,
        )
        .map_err(std::io::Error::other)?;

        let payload = wire;

        let destination_desc = reticulum::destination::DestinationDesc {
            identity: peer_info.identity,
            address_hash: AddressHash::new(destination),
            name: DestinationName::new("lxmf", "delivery"),
        };
        let transport = self.transport.clone();
        let receipt_map = self.receipt_map.clone();
        let message_id = record.id.clone();
        tokio::spawn(async move {
            let result = send_via_link(
                transport.as_ref(),
                destination_desc,
                &payload,
                std::time::Duration::from_secs(20),
            )
            .await;
            if let Ok(packet) = result {
                let packet_hash = hex::encode(packet.hash().to_bytes());
                track_receipt_mapping(&receipt_map, &packet_hash, &message_id);
            }
        });
        Ok(())
    }
}

impl AnnounceBridge for TransportBridge {
    fn announce_now(&self) -> Result<(), std::io::Error> {
        let transport = self.transport.clone();
        let destination = self.announce_destination.clone();
        let app_data = self.announce_app_data.clone();
        tokio::spawn(async move {
            transport
                .send_announce(&destination, app_data.as_deref())
                .await;
        });
        Ok(())
    }
}

fn parse_destination_hex(input: &str) -> Option<[u8; 16]> {
    let bytes = hex::decode(input).ok()?;
    if bytes.len() != 16 {
        return None;
    }
    let mut out = [0u8; 16];
    out.copy_from_slice(&bytes);
    Some(out)
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let local = LocalSet::new();
    local
        .run_until(async {
            let args = Args::parse();
            let addr: SocketAddr = args.rpc.parse().expect("invalid rpc address");
            let store = MessagesStore::open(&args.db).expect("open sqlite");

            let identity_path = args.identity.clone().unwrap_or_else(|| {
                let mut path = args.db.clone();
                path.set_extension("identity");
                path
            });
            let identity = load_or_create_identity(&identity_path).expect("load identity");
            let identity_hash = hex::encode(identity.address_hash().as_slice());
            let local_display_name = std::env::var("LXMF_DISPLAY_NAME")
                .ok()
                .and_then(|value| normalize_display_name(&value));
            let daemon_config = args.config.as_ref().and_then(|path| {
                match DaemonConfig::from_path(path) {
                    Ok(config) => Some(config),
                    Err(err) => {
                        eprintln!("[daemon] failed to load config {}: {}", path.display(), err);
                        None
                    }
                }
            });
            let mut configured_interfaces = daemon_config
                .as_ref()
                .map(|config| {
                    config
                        .interfaces
                        .iter()
                        .map(|iface| InterfaceRecord {
                            kind: iface.kind.clone(),
                            enabled: iface.enabled.unwrap_or(false),
                            host: iface.host.clone(),
                            port: iface.port,
                            name: iface.name.clone(),
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let mut transport: Option<Arc<Transport>> = None;
            let peer_crypto: Arc<std::sync::Mutex<HashMap<String, PeerCrypto>>> =
                Arc::new(std::sync::Mutex::new(HashMap::new()));
            let mut announce_destination: Option<Arc<tokio::sync::Mutex<SingleInputDestination>>> =
                None;
            let receipt_map: Arc<std::sync::Mutex<HashMap<String, String>>> =
                Arc::new(std::sync::Mutex::new(HashMap::new()));
            let (receipt_tx, mut receipt_rx) = unbounded_channel();

            if let Some(addr) = args.transport.clone() {
                let config = TransportConfig::new("daemon", &identity, true);
                let mut transport_instance = Transport::new(config);
                transport_instance
                    .set_receipt_handler(Box::new(ReceiptBridge::new(
                        receipt_map.clone(),
                        receipt_tx,
                    )))
                    .await;
                let iface_manager = transport_instance.iface_manager();
                iface_manager.lock().await.spawn(
                    TcpServer::new(addr.clone(), iface_manager.clone()),
                    TcpServer::spawn,
                );
                if let Some(config) = daemon_config.as_ref() {
                    for (host, port) in config.tcp_client_endpoints() {
                        let addr = format!("{}:{}", host, port);
                        iface_manager
                            .lock()
                            .await
                            .spawn(TcpClient::new(addr), TcpClient::spawn);
                        eprintln!(
                            "[daemon] tcp_client enabled name={} host={} port={}",
                            host, host, port
                        );
                    }
                }
                eprintln!("[daemon] transport enabled");
                if let Some((host, port)) = addr.rsplit_once(':') {
                    configured_interfaces.push(InterfaceRecord {
                        kind: "tcp_server".into(),
                        enabled: true,
                        host: Some(host.to_string()),
                        port: port.parse::<u16>().ok(),
                        name: Some("daemon-transport".into()),
                    });
                }

                let destination = transport_instance
                    .add_destination(identity.clone(), DestinationName::new("lxmf", "delivery"))
                    .await;
                {
                    let dest = destination.lock().await;
                    println!(
                        "[daemon] delivery destination hash={}",
                        hex::encode(dest.desc.address_hash.as_slice())
                    );
                }
                announce_destination = Some(destination);
                transport = Some(Arc::new(transport_instance));
            }

            let bridge: Option<Arc<TransportBridge>> = transport
                .as_ref()
                .zip(announce_destination.as_ref())
                .map(|(transport, destination)| {
                    Arc::new(TransportBridge::new(
                        transport.clone(),
                        identity.clone(),
                        destination.clone(),
                        local_display_name
                            .as_ref()
                            .map(|display_name| display_name.as_bytes().to_vec()),
                        peer_crypto.clone(),
                        receipt_map.clone(),
                    ))
                });

            let outbound_bridge: Option<Arc<dyn OutboundBridge>> = bridge
                .as_ref()
                .map(|bridge| bridge.clone() as Arc<dyn OutboundBridge>);
            let announce_bridge: Option<Arc<dyn AnnounceBridge>> = bridge
                .as_ref()
                .map(|bridge| bridge.clone() as Arc<dyn AnnounceBridge>);

            let daemon = Rc::new(RpcDaemon::with_store_and_bridges(
                store,
                identity_hash,
                outbound_bridge,
                announce_bridge,
            ));
            daemon.replace_interfaces(configured_interfaces);
            daemon.set_propagation_state(transport.is_some(), None, 0);

            if transport.is_some() {
                let daemon_receipts = daemon.clone();
                tokio::task::spawn_local(async move {
                    while let Some(event) = receipt_rx.recv().await {
                        let _ = handle_receipt_event(&daemon_receipts, event);
                    }
                });
            }

            if args.announce_interval_secs > 0 {
                let _handle = daemon
                    .clone()
                    .start_announce_scheduler(args.announce_interval_secs);
            }

            if let Some(transport) = transport.clone() {
                let daemon_inbound = daemon.clone();
                let inbound_transport = transport.clone();
                tokio::task::spawn_local(async move {
                    let mut rx = inbound_transport.received_data_events();
                    loop {
                        if let Ok(event) = rx.recv().await {
                            let data = event.data.as_slice();
                            eprintln!(
                                "[daemon] rx data len={} dst={}",
                                data.len(),
                                hex::encode(event.destination.as_slice())
                            );
                            let mut destination = [0u8; 16];
                            destination.copy_from_slice(event.destination.as_slice());
                            if let Some(record) = decode_inbound_payload(destination, data) {
                                let _ = daemon_inbound.accept_inbound(record);
                            }
                        }
                    }
                });

                let daemon_announce = daemon.clone();
                let peer_crypto = peer_crypto.clone();
                let announce_transport = transport.clone();
                tokio::task::spawn_local(async move {
                    let mut rx = announce_transport.recv_announces().await;
                    loop {
                        if let Ok(event) = rx.recv().await {
                            let dest = event.destination.lock().await;
                            let peer = hex::encode(dest.desc.address_hash.as_slice());
                            let identity = dest.desc.identity;
                            let (peer_name, peer_name_source) =
                                parse_peer_name_from_app_data(event.app_data.as_slice())
                                    .map(|(name, source)| (Some(name), Some(source.to_string())))
                                    .unwrap_or((None, None));
                            let _ratchet = event.ratchet;
                            peer_crypto
                                .lock()
                                .expect("peer map")
                                .insert(peer.clone(), PeerCrypto { identity });
                            if let Some(name) = peer_name.as_ref() {
                                eprintln!("[daemon] rx announce peer={} name={}", peer, name);
                            } else {
                                eprintln!("[daemon] rx announce peer={}", peer);
                            }
                            let timestamp = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|value| value.as_secs() as i64)
                                .unwrap_or(0);
                            let _ = daemon_announce.accept_announce_with_details(
                                peer,
                                timestamp,
                                peer_name,
                                peer_name_source,
                            );
                        }
                    }
                });
            }

            let listener = TcpListener::bind(addr).await.unwrap();
            println!("reticulumd listening on http://{}", addr);

            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buffer = Vec::new();
                loop {
                    let mut chunk = [0u8; 4096];
                    let read = stream.read(&mut chunk).await.unwrap();
                    if read == 0 {
                        break;
                    }
                    buffer.extend_from_slice(&chunk[..read]);
                    if let Some(header_end) = http::find_header_end(&buffer) {
                        let headers = &buffer[..header_end];
                        if let Some(length) = http::parse_content_length(headers) {
                            let body_start = header_end + 4;
                            if buffer.len() >= body_start + length {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }

                if buffer.is_empty() {
                    continue;
                }

                let response = http::handle_http_request(&daemon, &buffer).unwrap_or_else(|err| {
                    http::build_error_response(&format!("rpc error: {}", err))
                });
                let _ = stream.write_all(&response).await;
                let _ = stream.shutdown().await;
            }
        })
        .await;
}
