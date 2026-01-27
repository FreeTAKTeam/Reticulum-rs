use std::io;

use reticulum::destination::DestinationDesc;
use reticulum::destination::link::{LinkEvent, LinkStatus};
use reticulum::packet::Packet;
use reticulum::transport::Transport;
use tokio::time::{timeout, Duration, Instant};

pub async fn send_via_link(
    transport: &Transport,
    destination: DestinationDesc,
    payload: &[u8],
    wait_timeout: Duration,
) -> io::Result<Packet> {
    let link = transport.link(destination).await;
    let link_id = *link.lock().await.id();

    if link.lock().await.status() != LinkStatus::Active {
        let mut events = transport.out_link_events();
        let deadline = Instant::now() + wait_timeout;

        loop {
            if link.lock().await.status() == LinkStatus::Active {
                break;
            }

            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "link activation timed out",
                ));
            }

            match timeout(remaining, events.recv()).await {
                Ok(Ok(event)) => {
                    if event.id == link_id {
                        if let LinkEvent::Activated = event.event {
                            break;
                        }
                    }
                }
                Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => continue,
                Ok(Err(tokio::sync::broadcast::error::RecvError::Closed)) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "link event channel closed",
                    ));
                }
                Err(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "link activation timed out",
                    ));
                }
            }
        }
    }

    let packet = link
        .lock()
        .await
        .data_packet(payload)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("{:?}", err)))?;
    transport.send_packet(packet).await;

    Ok(packet)
}
