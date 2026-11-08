use bytes::BytesMut;
use futures_util::TryFutureExt;
use rtc::sansio::Protocol;
use rtc::shared::{TaggedBytesMut, TransportContext, TransportProtocol};
use rtc::stun::agent::StunEvent;
use rtc::stun::message::Getter;
use rtc::stun::{
    client::ClientBuilder,
    message::{BINDING_REQUEST, Message, TransactionId},
    xoraddr::XorMappedAddress,
};
use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};
use tokio::net::UdpSocket;

pub(super) async fn gather_srflx(
    socket: &UdpSocket,
    stun_server: &str,
) -> Result<SocketAddr, String> {
    let stun_remote = tokio::net::lookup_host(stun_server)
        .map_err(|e| e.to_string())
        .await?
        .next()
        .ok_or("STUN server not found")?;

    let local_addr = socket
        .local_addr()
        .map_err(|e| format!("local_addr error: {}", e))?;

    let mut client = ClientBuilder::new()
        .build(
            Instant::now(),
            local_addr,
            stun_remote,
            TransportProtocol::UDP,
        )
        .map_err(|e| format!("build STUN client: {e}"))?;

    // Create STUN Binding Request.
    let mut request = Message::new();

    request
        .build(&[Box::<TransactionId>::default(), Box::new(BINDING_REQUEST)])
        .map_err(|e| format!("build STUN request: {e}"))?;

    client
        .handle_write(rtc::stun::client::TaggedMessage {
            now: Instant::now(),
            message: request,
        })
        .map_err(|e| format!("STUN handle_write: {e}"))?;

    let mut buf = [0u8; 2048];
    let hard_deadline = Instant::now() + Duration::from_secs(8);

    loop {
        while let Some(pkt) = client.poll_write() {
            socket
                .send_to(&pkt.message, pkt.transport.peer_addr)
                .await
                .map_err(|e| format!("STUN send: {e}"))?;
        }

        // Process STUN events.
        while let Some(event) = client.poll_event() {
            match event {
                StunEvent::Message(msg) => {
                    let mut mapped = XorMappedAddress::default();

                    mapped
                        .get_from(&msg)
                        .map_err(|e| format!("extract XOR-MAPPED-ADDRESS: {e}"))?;

                    return Ok(SocketAddr::new(mapped.ip, mapped.port));
                }
                _ => {}
            }
            // let message = event.result.map_err(|e| format!("STUN transaction: {e}"))?;
            //
        }

        let now = Instant::now();

        if now >= hard_deadline {
            return Err("STUN gathering timed out".to_owned());
        }

        let deadline = client
            .poll_timeout()
            .unwrap_or(hard_deadline)
            .min(hard_deadline);

        let delay = deadline.saturating_duration_since(now);

        tokio::select! {
            result = socket.recv_from(&mut buf) => {
                let (n, peer_addr) =
                    result.map_err(|e| format!("STUN recv: {e}"))?;

                // Only accept response from the configured STUN server.
                if peer_addr != stun_remote {
                    continue;
                }

                client
                    .handle_read(TaggedBytesMut {
                        now: Instant::now(),
                        transport: TransportContext {
                            local_addr,
                            peer_addr,
                            ecn: None,
                            transport_protocol: TransportProtocol::UDP,
                        },
                        message: BytesMut::from(&buf[..n]),
                    })
                    .map_err(|e| format!("STUN handle_read: {e}"))?;
            }

            _ = tokio::time::sleep(delay) => {
                client
                    .handle_timeout(Instant::now())
                    .map_err(|e| format!("STUN handle_timeout: {e}"))?;
            }
        }
    }
}
