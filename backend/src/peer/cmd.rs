use crate::peer::{
    constant::{CONTROL_LABEL, MEDIA_QUEUE_SIZE, MESSAGE_LABEL},
    driver::{AttachedOutput, MediaReceiver, OutboundTrack, PeerDriver},
    error::PeerError,
    mux,
    worker::WorkerMessage,
};
use axum::extract::ws;
use futures_util::stream::SelectAll;
use rtc::{
    data_channel::RTCDataChannelInit,
    ice::{candidate::CandidateConfig, mdns::MulticastDnsMode, network_type::NetworkType},
    media_stream::MediaStreamTrack,
    peer_connection::{
        RTCPeerConnection, RTCPeerConnectionBuilder,
        configuration::{
            RTCConfigurationBuilder, RTCRtcpMuxPolicy,
            interceptor_registry::{Registry, register_default_interceptors},
            media_engine::MediaEngine,
            setting_engine::SettingEngineBuilder,
        },
        sdp::RTCSessionDescription,
        transport::{
            CandidateHostConfig, CandidateServerReflexiveConfig, RTCDtlsRole, RTCIceCandidateInit,
        },
    },
    rtp_transceiver::RTCRtpSenderId,
};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    rc::Rc,
    sync::Arc,
    time::Instant,
};
use tokio::sync::{mpsc, oneshot};
use tracing::debug;
use uuid::Uuid;

use crate::sfu::room::RoomSfu;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeerType {
    PUBLISHER,
    SUBSCRIBER,
}

impl From<PeerType> for String {
    fn from(value: PeerType) -> Self {
        match value {
            PeerType::PUBLISHER => "PUBLISHER".to_string(),
            PeerType::SUBSCRIBER => "SUBSCRIBER".to_string(),
        }
    }
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
        pc_type: PeerType,
    },
    /// register one outbound track; returns the wire-facing sender id and the
    /// packet queue that only this driver drains
    Attach {
        track: MediaStreamTrack,
        mime: String,
        resp: oneshot::Sender<Result<AttachedOutput, String>>,
    },
    Detach {
        sender_id: RTCRtpSenderId,
        resp: oneshot::Sender<Result<(), String>>,
    },
    SendControl(String),
    SendChat(String),
    Close,
}

#[derive(Clone)]
pub struct PeerCommandInstance {
    pub tx: mpsc::Sender<PeerCommand>,
}

impl PeerCommandInstance {
    pub async fn answer_offer(&self, sdp: String, pc_type: PeerType) -> Result<String, String> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(PeerCommand::AnswerOffer {
                sdp,
                resp: resp_tx,
                pc_type,
            })
            .await
            .map_err(|_| {
                PeerError::DriverError("peer driver unavailable".to_owned()).to_string()
            })?;
        resp_rx.await.map_err(|_| {
            PeerError::DriverError("peer driver dropped reply".to_owned()).to_string()
        })?
    }

    pub async fn apply_answer(&self, sdp: String) -> Result<(), String> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(PeerCommand::ApplyAnswer { sdp, resp: resp_tx })
            .await
            .map_err(|_| {
                PeerError::DriverError("peer driver unavailable".to_owned()).to_string()
            })?;
        resp_rx.await.map_err(|_| {
            PeerError::DriverError("peer driver dropped reply".to_owned()).to_string()
        })?
    }

    pub async fn add_remote_candidate(
        &self,
        candidate: String,
        sdp_mid: Option<String>,
        sdp_mline_index: Option<u16>,
        username_fragment: Option<String>,
        pc_type: PeerType,
    ) -> Result<(), String> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(PeerCommand::AddRemoteCandidate {
                init: RTCIceCandidateInit {
                    candidate,
                    sdp_mid,
                    sdp_mline_index,
                    username_fragment,
                    url: None,
                },
                resp: resp_tx,
                pc_type,
            })
            .await
            .map_err(|_| {
                PeerError::DriverError("peer driver unavailable".to_owned()).to_string()
            })?;
        resp_rx.await.map_err(|_| {
            PeerError::DriverError("peer driver dropped reply".to_owned()).to_string()
        })?
    }

    pub async fn attach(
        &self,
        track: MediaStreamTrack,
        mime: String,
    ) -> Result<AttachedOutput, String> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(PeerCommand::Attach {
                track,
                mime,
                resp: resp_tx,
            })
            .await
            .map_err(|_| {
                PeerError::DriverError("peer driver unavailable".to_owned()).to_string()
            })?;
        resp_rx.await.map_err(|_| {
            PeerError::DriverError("peer driver dropped reply".to_owned()).to_string()
        })?
    }

    pub async fn detach(&self, sender_id: RTCRtpSenderId) -> Result<(), String> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(PeerCommand::Detach {
                sender_id,
                resp: resp_tx,
            })
            .await
            .map_err(|_| {
                PeerError::DriverError("peer driver unavailable".to_owned()).to_string()
            })?;
        resp_rx.await.map_err(|_| {
            PeerError::DriverError("peer driver dropped reply".to_owned()).to_string()
        })?
    }

    pub async fn send_control(&self, json: String) {
        if self.tx.send(PeerCommand::SendControl(json)).await.is_err() {
            debug!("peer driver closed, control dropped");
        }
    }

    pub async fn send_chat(&self, json: String) {
        if self.tx.send(PeerCommand::SendChat(json)).await.is_err() {
            debug!("peer driver closed, chat dropped");
        }
    }

    pub async fn close(self) {
        let _ = self.tx.send(PeerCommand::Close).await;
    }
}

pub(super) async fn handle_msg(job: WorkerMessage) {
    match job {
        WorkerMessage::CreatePeer {
            announce_ip,
            rx,
            conn_id,
            room,
            ws_sink,
        } => {
            let peer = match build_peer(conn_id, announce_ip, room, rx, ws_sink).await {
                Ok(peer) => peer,
                Err(e) => {
                    tracing::error!("create peer: {e}");
                    return;
                }
            };
            tokio::task::spawn_local(async move {
                peer.run().await;
            });
        }
    }
}

async fn build_peer(
    conn_id: Uuid,
    announce_ip: IpAddr,
    room: Arc<RoomSfu>,
    rx: mpsc::Receiver<PeerCommand>,
    ws_sink: mpsc::Sender<ws::Message>,
) -> Result<PeerDriver, String> {
    let mux = mux::current();
    let local_addr = SocketAddr::new(announce_ip, mux.port);

    let mut publisher_pc = create_pc(local_addr).await?;
    let subscriber_pc = create_pc(local_addr).await?;

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
        conn_id,
        mux: Rc::clone(&mux),
        local_addr,
        net_rx,
        cmd_rx: rx,
        publisher_pc,
        subscriber_pc,
        ctrl_id,
        msg_id,
        inbound: HashMap::new(),
        outbound: HashMap::new(),
        media_rx: SelectAll::new(),
        offer_outstanding: false,
        pending_renegotiation: false,
        closing: false,
        room,
        ws_sink,
    })
}

async fn create_pc(local_addr: SocketAddr) -> Result<RTCPeerConnection, String> {
    let config = RTCConfigurationBuilder::default()
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

    // if let Some(srflx) = mux::current().srflx_addr {
    //     let mux = mux::current();

    //     let candidate = CandidateServerReflexiveConfig {
    //         base_config: CandidateConfig {
    //             network: "udp".to_owned(),
    //             address: srflx.ip().to_string(),
    //             port: srflx.port(),
    //             component: 1,
    //             ..Default::default()
    //         },
    //         rel_addr: local_addr.ip().to_string(),
    //         rel_port: mux.port,
    //         ..Default::default()
    //     }
    //     .new_candidate_server_reflexive()
    //     .map_err(|e| format!("build srflx candidate: {e}"))?;

    //     let local_candidate = rtc::peer_connection::transport::RTCIceCandidate::from(&candidate)
    //         .to_json()
    //         .map_err(|e| format!("candidate json: {e}"))?;

    //     pc.add_local_candidate(local_candidate)
    //         .map_err(|e| format!("add srflx candidate: {e}"))?;
    // }

    pc.add_local_candidate(RTCIceCandidateInit::default())
        .map_err(|e| format!("gather complete: {e}"))?;

    Ok(pc)
}

impl PeerDriver {
    pub(super) async fn parse_cmd(&mut self, cmd: PeerCommand) -> bool {
        let now = Instant::now();
        match cmd {
            PeerCommand::Close => return false,
            PeerCommand::AnswerOffer { sdp, resp, pc_type } => {
                let pc = match pc_type {
                    PeerType::PUBLISHER => &mut self.publisher_pc,
                    PeerType::SUBSCRIBER => &mut self.subscriber_pc,
                };
                let result = (|| -> Result<String, String> {
                    let offer = RTCSessionDescription::offer(sdp)
                        .map_err(|e| format!("parse offer: {e}"))?;
                    pc.set_remote_description(now, offer)
                        .map_err(|e| format!("remote desc: {e}"))?;
                    let answer = pc
                        .create_answer(None)
                        .map_err(|e| format!("create answer: {e}"))?;
                    pc.set_local_description(now, answer.clone())
                        .map_err(|e| format!("local desc: {e}"))?;
                    Ok(answer.sdp)
                })();

                if let Ok(sdp) = &result {
                    self.reset_outbound_cache();
                    self.bind_ufrag_from(sdp, pc_type);
                }
                let _ = resp.send(result);
            }
            PeerCommand::ApplyAnswer { sdp, resp } => {
                let result = (|| -> Result<(), String> {
                    let answer = RTCSessionDescription::answer(sdp)
                        .map_err(|e| format!("parse answer: {e}"))?;
                    self.publisher_pc
                        .set_remote_description(now, answer)
                        .map_err(|e| format!("remote answer desc: {e}"))?;
                    Ok(())
                })();
                self.offer_outstanding = false;
                match result {
                    Err(e) => {
                        let _ = resp.send(Err(e));
                    }
                    Ok(()) => {
                        self.reset_outbound_cache();
                        let _ = resp.send(Ok(()));
                        debug!("conn {}: answer applied", self.conn_id);

                        if self.pending_renegotiation {
                            self.pending_renegotiation = false;
                            self.send_offer_to_client().await;
                        }
                    }
                }
            }
            PeerCommand::AddRemoteCandidate {
                init,
                resp,
                pc_type,
            } => {
                let pc = match pc_type {
                    PeerType::PUBLISHER => &mut self.publisher_pc,
                    PeerType::SUBSCRIBER => &mut self.subscriber_pc,
                };
                let result = pc
                    .add_remote_candidate(init)
                    .map_err(|e| format!("add candidate: {e}"));
                let _ = resp.send(result);
            }
            PeerCommand::Attach { track, mime, resp } => {
                let result = (|| -> Result<AttachedOutput, String> {
                    let sender_id = self
                        .publisher_pc
                        .add_track(track)
                        .map_err(|e| format!("add track: {e}"))?;
                    let (tx, rx) = mpsc::channel(MEDIA_QUEUE_SIZE);
                    self.media_rx.push(MediaReceiver { sender_id, rx });
                    self.outbound.insert(
                        sender_id,
                        OutboundTrack {
                            mime,
                            pt: None,
                            ssrc: None,
                        },
                    );
                    Ok(AttachedOutput { sender_id, tx })
                })();
                let _ = resp.send(result);
            }
            PeerCommand::Detach { sender_id, resp } => {
                self.outbound.remove(&sender_id);
                let result = self
                    .publisher_pc
                    .remove_track(sender_id)
                    .map_err(|e| format!("remove track: {e}"));
                let _ = resp.send(result);
            }
            PeerCommand::SendControl(json) => {
                if let Some(mut dc) = self.publisher_pc.data_channel(self.ctrl_id) {
                    if let Err(e) = dc.send_text(now, json) {
                        debug!("conn {}: control send: {e}", self.conn_id);
                    }
                }
            }
            PeerCommand::SendChat(json) => {
                if let Some(mut dc) = self.publisher_pc.data_channel(self.msg_id) {
                    if let Err(e) = dc.send_text(now, json) {
                        debug!("conn {}: chat send: {e}", self.conn_id);
                    }
                }
            }
        }
        true
    }
}
