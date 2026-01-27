use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::LocalSet;

use reticulum::rpc::{http, RpcDaemon};
use reticulum::storage::messages::MessagesStore;

#[derive(Parser, Debug)]
#[command(name = "reticulumd")]
struct Args {
    #[arg(long, default_value = "127.0.0.1:4243")]
    rpc: String,
    #[arg(long, default_value = "reticulum.db")]
    db: PathBuf,
    #[arg(long, default_value_t = 0)]
    announce_interval_secs: u64,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let local = LocalSet::new();
    local
        .run_until(async {
            let args = Args::parse();
            let addr: SocketAddr = args.rpc.parse().expect("invalid rpc address");
            let store = MessagesStore::open(&args.db).expect("open sqlite");
            let daemon = Arc::new(RpcDaemon::with_store(store, "local".into()));
            if args.announce_interval_secs > 0 {
                let _handle = daemon.clone().start_announce_scheduler(args.announce_interval_secs);
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

                let response = http::handle_http_request(&daemon, &buffer)
                    .unwrap_or_else(|err| http::build_error_response(&format!("rpc error: {}", err)));
                let _ = stream.write_all(&response).await;
                let _ = stream.shutdown().await;
            }
        })
        .await;
}
