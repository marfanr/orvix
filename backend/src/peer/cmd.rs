use crate::peer::{
    constant::{CONTROL_LABEL, MESSAGE_LABEL},
    driver::PeerDriver,
    mux,
    worker::WorkerMessage,
};
use rtc::{
    data_channel::RTCDataChannelInit,
    ice::{candidate::CandidateConfig, mdns::MulticastDnsMode, network_type::NetworkType},
    media_stream::MediaStreamTrack,
    peer_connection::{
        RTCPeerConnection, RTCPeerConnectionBuilder,
        configuration::{
            RTCConfigurationBuilder, RTCIceServer, RTCRtcpMuxPolicy,
            interceptor_registry::{Registry, register_default_interceptors},
            media_engine::MediaEngine,
            setting_engine::SettingEngineBuilder,
        },
        transport::{
            CandidateHostConfig, CandidateServerReflexiveConfig, RTCDtlsRole, RTCIceCandidateInit,
        },
        sdp::{RTCSessionDescription}
    },
    rtp_transceiver::RTCRtpSenderId,
};
use std::{
    net::{IpAddr, SocketAddr},
    rc::Rc,
    time::Instant,
};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

pub enum PeerType {
    PUBLISHER,
    SUBSCRIBER,
}

pub enum PeerCommand {
    AnswerOffer {
        sdp: String,
        resp: oneshot::Sender<Result<String, String>>,
        pc_type: PeerType,
    },
    /// remote answer to one of our offers
    ApplyAnswer {
        sdp: String,
        resp: oneshot::Sender<Result<(), String>>,
    },
    AddRemoteCandidate {
        init: RTCIceCandidateInit,
        resp: oneshot::Sender<Result<(), String>>,
    },
    /// register one outbound track; returns the wire-facing sender id and the
    /// packet queue that only this driver drains
    Attach {
        track: MediaStreamTrack,
        mime: String,
        // resp: oneshot::Sender<Result<AttachedOutput, String>>,
    },
    Detach {
        sender_id: RTCRtpSenderId,
        resp: oneshot::Sender<Result<(), String>>,
    },
    SendControl(String),
    SendChat(String),
    Close,
}

pub struct PeerCommandInstance {
    pub tx: mpsc::Sender<PeerCommand>,
}

pub(super) async fn handle_msg(job: WorkerMessage) {
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
            peer.run().await;
        }
        _ => {}
    }
}

async fn build_peer(
    conn_id: Uuid,
    announce_ip: IpAddr,
    rx: mpsc::Receiver<PeerCommand>,
) -> Result<PeerDriver, String> {
    let mux = mux::current();
    let local_addr = SocketAddr::new(announce_ip, mux.port);

    let mut publisher_pc = create_pc(local_addr).await?;
    let mut subscriber_pc = create_pc(local_addr).await?;

    let ctrl = publisher_pc
        .create_data_channel(
            CONTROL_LABEL,
            Some(RTCDataChannelInit {
                negotiated: Some(0),
                ..Default::default()
            }),
        )
        .map_err(|e| format!("create control channel: {e}"))?;
    let ctrl_id = ctrl.id();

    let msg = publisher_pc
        .create_data_channel(
            MESSAGE_LABEL,
            Some(RTCDataChannelInit {
                negotiated: Some(1),
                ..Default::default()
            }),
        )
        .map_err(|e| format!("create message channel: {e}"))?;
    let msg_id = msg.id();

    let net_rx = mux.register(conn_id);

    Ok(PeerDriver {
        conn_id: conn_id,
        mux: Rc::clone(&mux),
        net_rx,
        cmd_rx: rx,
        publisher_pc,
        subscriber_pc,
    })
}

async fn create_pc(local_addr: SocketAddr) -> Result<RTCPeerConnection, String> {
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

    return Ok(pc);
}

impl PeerDriver {
    pub async fn parse_cmd(&self, cmd: PeerCommand) -> bool {
        let now = Instant::now();
        match cmd {
            PeerCommand::AnswerOffer { sdp, resp, pc_type } => {
                let pc = match pc_type {
                    PeerType::PUBLISHER => &self.subscriber_pc,
                    PeerType::SUBSCRIBER => &self.publisher_pc,
                };
                let offer =
                    RTCSessionDescription::offer(sdp).map_err(|e| format!("parse offer: {e}"))?;
                pc.set_remote_description(now, offer);

            }
            _ => {}
        }
        return true;
    }
}
