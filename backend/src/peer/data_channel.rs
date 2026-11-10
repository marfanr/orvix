// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

use crate::peer::constant::{FORWARD_RID, PLI_WINDOW};
use crate::peer::driver::PeerDriver;
use rtc::data_channel::RTCDataChannelMessage;
use rtc::peer_connection::message::RTCMessage;
use rtc::rtp_transceiver::rtp_sender::{RTCRtpCodec, RtpCodecKind};
use sonic_rs::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, error, info};
use uuid::Uuid;

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlMessage {
    Record,
    CodecChanged { codec: String, mime: String },
}

#[derive(Deserialize, Debug)]
pub struct DataMessage {
    pub target: String,
    pub msg: String,
    pub time: i64,
}

#[derive(Serialize, Debug)]
pub struct DataMessageResponse {
    pub target: String,
    pub msg: String,
    pub from: String,
    pub time: i64,
}

impl PeerDriver {
    async fn handle_control_message(&mut self, conn_id: Uuid, msg: RTCDataChannelMessage) {
        let json = sonic_rs::from_slice::<ControlMessage>(&msg.data);
        match json {
            Ok(ControlMessage::Record) => {
                info!("conn {conn_id}: received record control command via data channel");
                // start_record_bot(room.clone()).await;
                // self.room.tracks.r
            }
            Ok(ControlMessage::CodecChanged { codec, mime }) => {
                info!("codec changed into {}", codec);
                // for now only handle video

                let published = self
                    .room
                    .tracks
                    .get(&self.conn_id)
                    .map(|t| {
                        t.iter()
                            .find(|t| t.kind == RtpCodecKind::Video)
                            .map(Arc::clone)
                    })
                    .unwrap_or_default();

                let Some(track) = published else { return };

                let new_codec = RTCRtpCodec {
                    mime_type: mime.clone(),
                    ..Default::default()
                };

                track.switch_codec(new_codec.clone());

                for out in track.outputs.load_full().iter() {
                    if let Some(peer) = self.room.peers.get(&out.sub) {
                        let inst = peer.peer.clone();
                        drop(peer);
                        inst.set_outbound_codec(out.sender_id, mime.clone()).await;
                    }
                }

                if let Some(state) = self
                    .inbound
                    .values_mut()
                    .find(|t| t.kind == RtpCodecKind::Video)
                {
                    state.paused = false;
                    state.pli_until = Instant::now() + PLI_WINDOW;
                }

                let layers: Vec<(String, u32)> = track
                    .simulcast_layers
                    .iter()
                    .map(|t| (t.key().clone(), *t.value()))
                    .collect();

                self.room.unpublish_track(&track).await;

                for (rid, ssrc) in layers {
                    self.room
                        .publish_track(conn_id, RtpCodecKind::Video, new_codec.clone(), rid, ssrc)
                        .await;
                }
            }
            Err(e) => {
                debug!("conn {conn_id}: control parse err: {e}");
            }
        }
    }

    pub(super) async fn handle_inbound_message(&mut self, now: Instant, message: RTCMessage) {
        match message {
            RTCMessage::RtpPacket(track_id, pkt) => {
                let (kind, rid, resumed) = {
                    let Some(t) = self.inbound.get_mut(&track_id) else {
                        return;
                    };
                    t.last_rtp = now;
                    let rid = t
                        .layers
                        .get(&pkt.header.ssrc)
                        .cloned()
                        .unwrap_or_else(|| "default".to_string());

                    let mut resumed = false;
                    if t.paused {
                        t.paused = false;
                        t.pli_until = now + PLI_WINDOW;
                        resumed = true;
                        info!("conn {}: track resumed", self.conn_id);
                    }
                    (t.kind, rid, resumed)
                };

                // fan out only the forwarded layer; h/f layers stay inbound
                if rid != FORWARD_RID {
                    return;
                }

                // clone before awaiting so no DashMap guard is held across the await
                let published = self
                    .room
                    .tracks
                    .get(&self.conn_id)
                    .map(|ts| ts.iter().find(|t| t.kind == kind).map(Arc::clone))
                    .unwrap_or_default();
                if let Some(published) = published {
                    if resumed {
                        self.room.set_track_active(&published, true).await;
                    }
                    published.forward(&pkt).await;
                }
            }
            RTCMessage::RtcpPacket(_, _) => {
                // NACK/reports are handled by interceptors
            }
            RTCMessage::DataChannelMessage(channel_id, msg) => {
                if channel_id == self.sub_ctrl_id {
                    self.handle_control_message(self.conn_id, msg).await;
                } else if channel_id == self.sub_msg_id {
                    self.handle_chat_message(self.conn_id, msg).await;
                } else {
                    debug!(
                        "conn {}: data channel {channel_id} unknown, message dropped",
                        self.conn_id
                    );
                }
            }
            _ => {}
        }
    }

    async fn handle_chat_message(&mut self, conn_id: Uuid, msg: RTCDataChannelMessage) {
        let json = match sonic_rs::from_slice::<DataMessage>(&msg.data) {
            Ok(json) => json,
            Err(e) => {
                error!("conn {conn_id}: message data channel err: {e}");
                return;
            }
        };

        println!("msg : {}", str::from_utf8(&msg.data).unwrap());
        let username = self
            .room
            .peers
            .get(&conn_id)
            .and_then(|p| Some(p.username.to_owned()))
            .expect("username must be filled");

        let resp = DataMessageResponse {
            from: username.to_string(),
            msg: json.msg,
            target: json.target,
            time: json.time,
        };
        self.room.broadcast_chat(conn_id, &resp).await;
    }
}
