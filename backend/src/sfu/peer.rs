// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

use crate::sfu::PeerType;
use crate::sfu::data_channel::handle_data_channel;
use crate::sfu::room::{RoomSfu, renegotiate};
use axum::extract::ws::Message;
use futures_util::{FutureExt, select};
use rtc::ice::network_type::NetworkType;
use rtc::interceptor::Registry;
use webrtc::peer_connection::RTCRtcpMuxPolicy;
use rtc::peer_connection::configuration::interceptor_registry::{
    configure_simulcast_extension_headers, register_default_interceptors,
};
use rtc::rtp_transceiver::rtp_sender::RtpCodecKind;
use rtc::rtp_transceiver::{RTCRtpTransceiverDirection, RTCRtpTransceiverInit};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use rtc::ice::mdns::MulticastDnsMode;
use tokio::sync::mpsc;
use tokio::time::{Instant, timeout};
use tracing::{info, warn};
use uuid::Uuid;
use webrtc::data_channel::{DataChannel, DataChannelEvent};
use webrtc::media_stream::track_remote::{TrackRemote, TrackRemoteEvent};
use webrtc::peer_connection::crypto::providers::RingProvider;
use webrtc::peer_connection::{
    MediaEngine, PeerConnection, PeerConnectionBuilder, PeerConnectionEventHandler,
    RTCConfigurationBuilder, RTCIceConnectionState, RTCIceServer, RTCPeerConnectionIceErrorEvent,
    RTCPeerConnectionIceEvent, SettingEngineBuilder,
};
use webrtc::runtime::TokioRuntime;

static PROVIDER: OnceLock<Arc<RingProvider>> = OnceLock::new();

const FORWARD_RID: &str = "q";

pub async fn create_peer(
    conn_id: Uuid,
    room: Arc<RoomSfu>,
    self_sink: Option<mpsc::Sender<Message>>,
    peer_type: PeerType,
) -> Result<Arc<dyn PeerConnection>, String> {
    let provider = PROVIDER
        .get()
        .map(|provider| Arc::clone(provider))
        .unwrap_or_else(|| {
            let p = Arc::new(RingProvider::new());
            _ = PROVIDER.set(Arc::clone(&p));
            p.clone()
        });

    let config = RTCConfigurationBuilder::default()
        .with_ice_servers(vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }])
        .with_rtcp_mux_policy(RTCRtcpMuxPolicy::Require)
        .build();

    let se = SettingEngineBuilder::default()
        .with_crypto_provider(provider.clone())
        .with_allow_insecure_verification_algorithm(false)
        .with_multicast_dns_mode(MulticastDnsMode::Disabled)
        .with_network_types(vec![NetworkType::Udp4])
        .build();


    let mut media_engine = MediaEngine::default();
    media_engine
        .register_default_codecs()
        .map_err(|e| format!("register codecs: {e}"))?;

    configure_simulcast_extension_headers(&mut media_engine)
        .map_err(|e| format!("configure_simulcast_extension_headers: {e}"))?;

    let reg_ = Registry::new();

    let reg = register_default_interceptors(reg_, &mut media_engine)
        .map_err(|e| format!("register_default_interceptors: {e}"))?;

    let pc = PeerConnectionBuilder::new()
        .with_configuration(config)
        .with_media_engine(media_engine)
        .with_interceptor_registry(reg)
        .with_runtime(Arc::new(TokioRuntime))
        .with_handler(Arc::new(PcHandler {
            conn_id,
            ws_conn: self_sink,
            room: room.clone(),
            peer_type,
        }))
        .with_udp_addrs((vec!["0.0.0.0:0"]))
        .with_setting_engine(se)
        .build()
        .await
        .map_err(|e| format!("build pc: {e}"))?;

    Ok(Arc::new(pc))
}

pub struct PcHandler {
    pub conn_id: Uuid,
    pub ws_conn: Option<mpsc::Sender<Message>>,
    pub room: Arc<RoomSfu>,
    pub peer_type: PeerType,
}

#[async_trait::async_trait]
impl PeerConnectionEventHandler for PcHandler {
    async fn on_ice_candidate(&self, event: RTCPeerConnectionIceEvent) {
        let init = match event.candidate.to_json() {
            Ok(t) => t,
            Err(e) => {
                warn!("candidate to_json: {e}");
                return;
            }
        };
        let port = init.candidate.split_whitespace().nth(5).unwrap_or("");
        info!("ice detected port {} from {:#?}", port, self.peer_type);

        let peer_type = self.peer_type.to_string();

        let msg = sonic_rs::json!({
            "type": "ice",
            "peer_type": peer_type,
            "candidate": &init.candidate,
            "sdpMid": &init.sdp_mid,
            "sdpMLineIndex": &init.sdp_mline_index,
            "usernameFragment": &init.username_fragment,
        })
            .to_string();

        if let Some(ws_conn) = self.ws_conn.clone() {
            if ws_conn.send(Message::Text(msg.into())).await.is_err() {
                warn!("ws closed, drop ice candidate");
            }
        }
    }

    async fn on_ice_candidate_error(&self, event: RTCPeerConnectionIceErrorEvent) {
        warn!("ice candidate error: {:?}", event);
    }

    async fn on_ice_connection_state_change(&self, state: RTCIceConnectionState) {
        info!("on ice state change {}", state.clone());
        if self.peer_type.is_publisher() {
            return;
        }
        if state == RTCIceConnectionState::Connected {
            println!("time to setupped publisher");
            if let Some(curr_peer) = self.room.peers.get(&self.conn_id) {
                let peer = Arc::clone(curr_peer.value());
                renegotiate(&peer).await;
            }
        }
    }

    async fn on_data_channel(&self, dc: Arc<dyn DataChannel>) {
            let conn_id = self.conn_id;
            let room = Arc::clone(&self.room);

            tokio::spawn(Box::pin(async move {
                let label = Arc::from(dc.label().await.unwrap_or_default());
                loop {
                    select! {
                    event = dc.poll().fuse() => {
                        match event {
                            Some(DataChannelEvent::OnMessage(msg)) =>
                                handle_data_channel(
                                    conn_id,
                                    msg,
                                    Arc::clone(&label),
                                    room.clone()
                                ).await,
                            Some(DataChannelEvent::OnClose) | None => break,
                            _ => {}
                        }
                    }
                }
                }
            }));
        }

        async fn on_track(&self, track: Arc<dyn TrackRemote>) {
            println!(
                "found track ({}) from {}",
                track.ssrcs().await.len(),
                self.peer_type.to_string()
            );
            if track.ssrcs().await.is_empty() {
                warn!("track without ssrc");
                return;
            }

            let con_id = self.conn_id.clone();
            let room = self.room.clone();
            let kind = track.kind().await;

            let mut published = None;
            let is_video = kind == rtc::rtp_transceiver::rtp_sender::RtpCodecKind::Video;

            tokio::spawn(async move {
                let idle_timeout = Duration::from_millis(500);
                let pli_interval = Duration::from_millis(250);
                let pli_window = Duration::from_secs(3);

                let mut pli_until = Instant::now() + pli_window;
                let mut last_pli: Option<Instant> = None;
                let mut last_rtp = Instant::now();

                let mut selected: Option<u32> = None;
                let mut last_outputs = 0usize;

                loop {
                    match timeout(idle_timeout, track.poll()).await {
                        Ok(Some(TrackRemoteEvent::OnOpen(init))) => {
                            let ssrc = init.ssrc;

                            let rid = init.rid.as_deref().unwrap_or(FORWARD_RID).to_owned();
                            info!("NEW TRACK=conn {con_id}: layer opened rid={rid:?} ssrc={ssrc}");

                            if rid != FORWARD_RID {
                                continue;
                            }

                            let Some(codec) = track.codec(ssrc).await else {
                                continue;
                            };

                            selected = Some(ssrc);

                            info!("publisher conn {con_id}");
                            let p = room.publish_track(con_id, kind, codec, rid, ssrc).await;
                            p.set_remote_track(Arc::clone(&track));
                            published = Some(p);
                        }
                        Ok(Some(TrackRemoteEvent::OnRtpPacket(pkt))) => {
                            let Some(p) = published.as_ref() else {
                                continue;
                            };

                            let Some(fwd_ssrc) = p.simulcast_layers.get(FORWARD_RID).map(|v| *v) else {
                                continue;
                            };

                            if pkt.header.ssrc != fwd_ssrc {
                                continue;
                            }

                            last_rtp = Instant::now();
                            if room.set_track_active(p, true).await {
                                pli_until = Instant::now() + pli_window;
                                info!("conn {con_id}: track resumed");
                            }
                            p.forward(&pkt).await;
                        }
                        Ok(Some(TrackRemoteEvent::OnEnded)) => {
                            info!("conn {con_id}: track layer ended explicitly");
                            break;
                        }
                        Ok(None) => {
                            info!("conn {con_id}: stream channel closed");
                            break;
                        }
                        Ok(Some(_)) | Err(_) => {}
                    }

                    let (Some(p), Some(active_ssrc)) = (published.as_ref(), selected) else {
                        continue;
                    };

                    if last_rtp.elapsed() > idle_timeout && room.set_track_active(p, false).await {
                        info!("conn {con_id}: active layer idle, paused track");
                    }

                    // subscriber baru: ulangi burst PLI supaya keyframe tidak hilang
                    // sebelum subscriber selesai negosiasi
                    let outputs_now = p.outputs.load().len();
                    if outputs_now > last_outputs {
                        pli_until = Instant::now() + pli_window;
                    }
                    last_outputs = outputs_now;

                    let wants_keyframe = p.is_recording() && !p.has_keyframe();

                    if is_video
                        && p.is_active()
                        && (Instant::now() < pli_until || wants_keyframe)
                        && last_pli.map_or(true, |t| t.elapsed() >= pli_interval)
                    {
                        let pli = Box::new(
                            rtc::rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication {
                                sender_ssrc: 0,
                                media_ssrc: active_ssrc,
                            },
                        );
                        if let Err(e) = track.write_rtcp(vec![pli]).await {
                            warn!("conn {con_id}: failed to send PLI: {e}");
                        }
                        last_pli = Some(Instant::now());
                    }
                }

                // CLEANUP SAAT LAYER FORWARD TERPUTUS
                if let Some(p) = published.as_ref() {
                    if let Some(sel) = selected {
                        p.simulcast_layers.retain(|_, v| *v != sel);
                    }

                    if p.simulcast_layers.is_empty() {
                        room.unpublish_track(p).await;
                        info!("conn {con_id}: all layers dropped, track successfully unpublished");
                    }
                }
            });
        }
    }
