// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

use crate::sfu::room_peer::RoomPeer;
use crate::sfu::svc::{LayerSelector, Verdict, parse_vp9};
use crate::utils::describe_nal;
use arc_swap::{ArcSwap, ArcSwapOption};
use dashmap::DashMap;
use rtc::media_stream::MediaStreamTrack;
use rtc::rtp::Packet;
use rtc::rtp_transceiver::RTCRtpSenderId;
use rtc::rtp_transceiver::rtp_sender::{
    RTCRtpCodec, RTCRtpCodingParameters, RTCRtpEncodingParameters, RtpCodecKind,
};
use sonic_rs::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, atomic};
use tokio::sync::mpsc;
use tracing::{debug, warn};
use uuid::Uuid;

/// One subscriber leg of a published track. `sender_id` refers to the `RTCRtpSender`
/// inside the *subscriber's* peer driver, `tx` is that driver's packet queue
/// (`AttachedOutput.tx` from `PeerCommandInstance::attach`).
#[derive(Clone)]
pub struct Output {
    pub sub: Uuid,
    pub sender_id: RTCRtpSenderId,
    pub tx: mpsc::Sender<Packet>,
    pub svc: Arc<LayerSelector>,
}

pub struct RecordingHandle {
    pub tx: mpsc::Sender<Packet>,
}

#[derive(Debug, Serialize)]
struct VoiceActivity {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub from: Uuid,
    pub active: bool,
}

pub struct PublishedTrack {
    pub publisher: Uuid,
    pub kind: RtpCodecKind,
    pub codec: ArcSwap<RTCRtpCodec>,
    pub outputs: ArcSwap<Vec<Output>>,
    pub active: AtomicBool,
    recorder: ArcSwapOption<RecordingHandle>,
    has_keyframe: AtomicBool,
    pub simulcast_layers: DashMap<String, u32>,
    pub sender_peer: Arc<RoomPeer>,
    pub track_id: String,
    recent_voice_activity: AtomicBool,
}

impl PublishedTrack {
    pub fn new(
        publisher: Uuid,
        kind: RtpCodecKind,
        codec: RTCRtpCodec,
        romm_peer: Arc<RoomPeer>,
        track_id: String,
    ) -> PublishedTrack {
        Self {
            publisher,
            kind,
            codec: ArcSwap::from_pointee(codec),
            outputs: ArcSwap::from_pointee(Vec::new()),
            // A track only reaches this state after its first RTP packet arrived,
            // so the publisher is considered active until proven otherwise.
            active: AtomicBool::new(true),
            recorder: ArcSwapOption::empty(),
            has_keyframe: AtomicBool::new(false),
            simulcast_layers: DashMap::new(),
            sender_peer: romm_peer,
            track_id,
            recent_voice_activity: AtomicBool::new(true),
        }
    }

    pub fn switch_codec(&self, codec: RTCRtpCodec) {
        self.codec.store(Arc::new(codec));
        self.has_keyframe.store(false, Ordering::Relaxed);
    }

    pub fn add_layer(&self, rid: String, ssrc: u32) {
        self.simulcast_layers.insert(rid, ssrc);
    }

    /// Whether the publisher is currently sending media for this track.
    /// This is the single source of truth for the mute state of a track.
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    pub fn attach_recorder(&self, handle: RecordingHandle) {
        self.has_keyframe.store(false, Ordering::Relaxed);
        self.recorder.store(Some(Arc::new(handle)));
    }

    pub fn stop_recording(&self) {
        self.recorder.store(None);
    }

    pub fn is_recording(&self) -> bool {
        self.recorder.load().is_some()
    }

    pub fn has_keyframe(&self) -> bool {
        self.has_keyframe.load(Ordering::Relaxed)
    }

    fn is_keyframe(payload: &[u8]) -> bool {
        let Some(f) = payload.first().map(|f| f.clone()) else {
            return false;
        };
        let ntype = oxideav_bitstream::h264::nal_header(f).2;
        match ntype {
            oxideav_bitstream::h264::NAL_TYPE_IDR => true,
            28 => payload.get(2).map_or(false, |&b| (b & 0x1f) == 5),
            _ => false,
        }
    }

    /// Fresh outbound `MediaStreamTrack` for `PeerCommandInstance::attach`. The
    /// peer driver owns payload/SSRC rewriting in `PeerDriver::write_to_sender`,
    /// so this only needs to carry kind + codec + one encoding with an SSRC.
    pub fn media_track(&self) -> MediaStreamTrack {
        let ssrc = Uuid::new_v4().as_u128() as u32;
        let codec: RTCRtpCodec = self.codec.load_full().as_ref().clone();
        MediaStreamTrack::new(
            format!("stream-{}", self.publisher),
            format!("track-{}-{}", self.publisher, self.kind),
            format!("label-{}-{}", self.publisher, self.kind),
            self.kind,
            vec![RTCRtpEncodingParameters {
                rtp_coding_parameters: RTCRtpCodingParameters {
                    ssrc: Some(ssrc),
                    ..Default::default()
                },
                codec: codec,
                ..Default::default()
            }],
        )
    }

    /// Hot path: called per RTP packet from publisher.
    pub async fn forward(&self, pkt: &Packet) {
        if let Some(rec) = self.recorder.load_full() {
            match rec.tx.try_send(pkt.clone()) {
                Ok(()) => {}
                Err(_) => debug!("recorder queue full/closed for {}", self.publisher),
            }
        }

        if Self::is_keyframe(&pkt.payload) {
            self.has_keyframe.store(true, Ordering::Relaxed);
        }

        if let Some(desc) = describe_nal(&pkt.payload) {
            debug!("nal: {desc}");
        }

        let q_ssrc = match self.simulcast_layers.get("q") {
            Some(val) => *val,
            None => return,
        };

        if pkt.header.ssrc != q_ssrc {
            return;
        }

        let audio_lvl_ext_id = self
            .sender_peer
            .audio_lvl_ext_id
            .load(atomic::Ordering::Relaxed);

        if self.kind == RtpCodecKind::Audio && audio_lvl_ext_id > 0 {
            if let Some(&value) = pkt
                .header
                .get_extension(audio_lvl_ext_id)
                .as_deref()
                .filter(|ex| ex.len() == 1)
                .and_then(|ex| ex.first())
            {
                let voice_activity = (value & 0x80) != 0;
                let audio_level = value & 0x7F;
                // rms
                let levl = 10f32.powf(-(audio_level as f32) / 20.0);
                let conn_id = self.sender_peer.conn_id;
                let active = levl > 0.03 && voice_activity;
                if self.recent_voice_activity.swap(active, Ordering::Relaxed) != active {
                    _ = self
                        .sender_peer
                        .room
                        .broadcast_control(
                            Uuid::nil(),
                            &VoiceActivity {
                                event_type: "voice_activity",
                                from: conn_id,
                                active: active,
                            },
                        )
                        .await;
                }
            }
        }

        let d = parse_vp9(&pkt.payload);
        let isvp9 = self.codec.load().mime_type.eq("video/VP9");
        let mut needs_keyframe = false;

        for o in self.outputs.load_full().iter() {
            if !isvp9 {
                match o.tx.try_send(pkt.clone()) {
                    Ok(()) => {}
                    Err(_) => {
                        warn!("forward {} -> {}: fanout queue full", self.publisher, o.sub)
                    }
                }
                continue;
            }
            let verdict = match &d {
                Some(d) => o
                    .svc
                    .select(pkt.header.sequence_number, pkt.header.marker, d),
                None => Verdict::Drop,
            };
            match verdict {
                Verdict::Forward { seq, marker } => {
                    let mut p = pkt.clone();
                    p.header.sequence_number = seq;
                    p.header.marker = marker;

                    match o.tx.try_send(p) {
                        Ok(()) => {}
                        Err(_) => {
                            warn!("forward {} -> {}: fanout queue full", self.publisher, o.sub)
                        }
                    }
                }
                Verdict::DropNeedKeyframe => {
                    needs_keyframe = true;
                }
                Verdict::Drop => {}
            }
        }

        // one PLI request per packet is enough: every output of this track
        // shares the same publisher keyframe
        if needs_keyframe {
            _ = self
                .sender_peer
                .peer
                .pli_wanted(self.track_id.clone())
                .await;
            debug!("requested keyframe for {}", self.publisher);
        }
    }

    pub async fn drop_output(&self, sub: Uuid) {
        self.outputs.rcu(|cur| {
            cur.iter()
                .filter(|o| o.sub != sub)
                .cloned()
                .collect::<Vec<_>>()
        });
    }

    pub fn is_output_exist(&self, sub: Uuid) -> bool {
        self.outputs.load().iter().any(|o| o.sub == sub)
    }

    pub fn renew_outputs(&self) -> Arc<Vec<Output>> {
        self.outputs.swap(Arc::new(Vec::new()))
    }

    pub async fn add_output(&self, o: Output) {
        self.outputs.rcu(|out| {
            let mut n = Vec::clone(out);
            n.push(o.clone());
            n
        });
    }
}
