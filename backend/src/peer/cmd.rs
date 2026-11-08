use crate::peer::{
    constant::{CONTROL_LABEL, MESSAGE_LABEL},
    driver::PeerDriver,
    mux,
    worker::{PeerWorker, WorkerMessage},
};
use rtc::{
    data_channel::RTCDataChannelInit,
    ice::{candidate::CandidateConfig, mdns::MulticastDnsMode, network_type::NetworkType},
    peer_connection::{
        RTCPeerConnectionBuilder,
        configuration::{
            RTCConfigurationBuilder, RTCIceServer, RTCRtcpMuxPolicy,
            interceptor_registry::{Registry, register_default_interceptors},
            media_engine::MediaEngine,
            setting_engine::SettingEngineBuilder,
        },
        transport::{
            CandidateHostConfig, CandidateServerReflexiveConfig, RTCDtlsRole, RTCIceCandidateInit,
        },
    },
};
use uuid::Uuid;
use std::{
    net::{IpAddr, SocketAddr}, rc::Rc, time::Instant,
};
use tokio::{net::unix::pipe::Receiver, sync::mpsc};

pub enum PeerCommand {}

pub struct PeerCommandInstance {
    pub tx: mpsc::Sender<PeerCommand>,
}

pub(super) async fn handle_msg(job: &WorkerMessage) {
    match job {
        WorkerMessage::CreatePeer {
            announce_ip,
            rx,
            conn_id,
        } => {
            let peer = match build_peer(conn_id, announce_ip.clone(), rx).await {
                Ok(peer) => peer,
                Err(e) => {
                    tracing::error!("create peer: {e}");
                    return;
                }
            };
        }
        _ => {}
    }
}

async fn build_peer(conn_id: &Uuid, announce_ip: IpAddr, rx: &mpsc::Receiver<PeerCommand>) -> Result<PeerDriver, String> {
    let mux = mux::current();
    let local_addr = SocketAddr::new(announce_ip, mux.port);

    let config = RTCConfigurationBuilder::default()
        .with_ice_servers(vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }])
        .with_rtcp_mux_policy(RTCRtcpMuxPolicy::Require)
        .build();

    let setting_engine = SettingEngineBuilder::new()
        .with_multicast_dns_mode(MulticastDnsMode::Disabled)
        .with_network_types(vec![NetworkType::Udp4])
        .with_answering_dtls_role(RTCDtlsRole::Server)
        .build();

    let mut media_engine = MediaEngine::default();
    media_engine
        .register_default_codecs()
        .map_err(|e| format!("register codecs: {e}"))?;

    let registry = register_default_interceptors(Registry::new(), &mut media_engine)
        .map_err(|e| format!("register interceptors: {e}"))?;

    let mut pc = RTCPeerConnectionBuilder::new()
        .with_configuration(config)
        .with_setting_engine(setting_engine)
        .with_media_engine(media_engine)
        .with_interceptor_registry(registry)
        .build(Instant::now())
        .map_err(|e| format!("build pc: {e}"))?;

    // host candidate
    let candidate = CandidateHostConfig {
        base_config: CandidateConfig {
            network: "udp".to_owned(),
            address: local_addr.ip().to_string(),
            port: local_addr.port(),
            component: 1,
            ..Default::default()
        },
        ..Default::default()
    }
    .new_candidate_host()
    .map_err(|e| format!("host candidate: {e}"))?;

    let local_candidate = rtc::peer_connection::transport::RTCIceCandidate::from(&candidate)
        .to_json()
        .map_err(|e| format!("candidate json: {e}"))?;

    pc.add_local_candidate(local_candidate)
        .map_err(|e| format!("add candidate: {e}"))?;

    // stun candidate
    if let Some(srflx) = mux::current().srflx_addr {
        let mux = mux::current();

        let candidate = CandidateServerReflexiveConfig {
            base_config: CandidateConfig {
                network: "udp".to_owned(),
                address: srflx.ip().to_string(),
                port: srflx.port(),
                component: 1,
                ..Default::default()
            },
            rel_addr: local_addr.ip().to_string(),
            rel_port: mux.port,
            ..Default::default()
        }
        .new_candidate_server_reflexive()
        .map_err(|e| format!("build srflx candidate: {e}"))?;

        let local_candidate = rtc::peer_connection::transport::RTCIceCandidate::from(&candidate)
            .to_json()
            .map_err(|e| format!("candidate json: {e}"))?;

        pc.add_local_candidate(local_candidate)
            .map_err(|e| format!("add srflx candidate: {e}"))?;
    }

    // empty candidate = end-of-gathering sentinel
    pc.add_local_candidate(RTCIceCandidateInit::default())
        .map_err(|e| format!("gather complete: {e}"))?;

    // out-of-band negotiated channels, matching the client side ids 0 and 1
    let ctrl = pc
        .create_data_channel(
            CONTROL_LABEL,
            Some(RTCDataChannelInit {
                negotiated: Some(0),
                ..Default::default()
            }),
        )
        .map_err(|e| format!("create control channel: {e}"))?;
    let ctrl_id = ctrl.id();

    let msg = pc
        .create_data_channel(
            MESSAGE_LABEL,
            Some(RTCDataChannelInit {
                negotiated: Some(1),
                ..Default::default()
            }),
        )
        .map_err(|e| format!("create message channel: {e}"))?;
    let msg_id = msg.id();


    let conn_id = conn_id.clone();
    let net_rx = mux.register(conn_id);

    Ok((PeerDriver {
        conn_id: conn_id,
        mux: Rc::clone(&mux)
    }))
}
