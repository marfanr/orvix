use bytes::BytesMut;
use rtc::sansio::Protocol;
use rtc::shared::{TaggedBytesMut, TransportContext, TransportProtocol};
use rtc::turn::client::{Client, ClientConfig, Event};
use rtc::turn::crypto::RTCCryptoProvider;
use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::net::UdpSocket;

pub(crate) struct TurnCredentials<'a> {
    pub username: &'a str,
    pub password: &'a str,
    pub realm: &'a str,
}

pub(crate) async fn gather_relay(
    socket: &UdpSocket,
    turn_server: &str,
    cred: TurnCredentials<'_>,
    crypto_provider: Arc<dyn RTCCryptoProvider>,
) -> Result<SocketAddr, String> {
    let turn_remote = tokio::net::lookup_host(turn_server)
        .await
        .map_err(|e| format!("resolve TURN server {turn_server}: {e}"))?
        .find(SocketAddr::is_ipv4)
        .ok_or_else(|| format!("TURN server {turn_server} has no IPv4 address"))?;

    let local_addr = socket
        .local_addr()
        .map_err(|e| format!("local_addr: {e}"))?;

    let mut client = Client::new(
        ClientConfig {
            stun_serv_addr: turn_remote.to_string(),
            turn_serv_addr: turn_remote.to_string(),
            local_addr,
            transport_protocol: TransportProtocol::UDP,
            username: cred.username.to_owned(),
            password: cred.password.to_owned(),
            realm: cred.realm.to_owned(),
            software: String::new(),
            ..Default::default()
        },
        crypto_provider,
    )
    .map_err(|e| format!("build TURN client: {e}"))?;

    client
        .allocate(Instant::now())
        .map_err(|e| format!("TURN allocate: {e}"))?;

    let mut buf = [0u8; 2048];
    let hard_deadline = Instant::now() + Duration::from_secs(8);

    loop {
        while let Some(pkt) = client.poll_write() {
            socket
                .send_to(&pkt.message, pkt.transport.peer_addr)
                .await
                .map_err(|e| format!("TURN send: {e}"))?;
        }

        while let Some(event) = client.poll_event() {
            match event {
                Event::AllocateResponse(_, relayed) => return Ok(relayed),
                Event::AllocateError(_, e) => return Err(format!("TURN allocate failed: {e}")),
                _ => {}
            }
        }

        let now = Instant::now();
        if now >= hard_deadline {
            return Err("TURN allocation timed out".to_owned());
        }
        let deadline = client
            .poll_timeout()
            .unwrap_or(hard_deadline)
            .min(hard_deadline);
        let delay = deadline.saturating_duration_since(now);

        tokio::select! {
            result = socket.recv_from(&mut buf) => {
                let (n, peer_addr) = result.map_err(|e| format!("TURN recv: {e}"))?;
                if peer_addr != turn_remote {
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
                    .map_err(|e| format!("TURN handle_read: {e}"))?;
            }
            _ = tokio::time::sleep(delay) => {
                client
                    .handle_timeout(Instant::now())
                    .map_err(|e| format!("TURN handle_timeout: {e}"))?;
            }
        }
    }
}
