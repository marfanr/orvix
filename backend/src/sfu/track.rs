// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

use crate::utils::describe_nal;
use arc_swap::{ArcSwap, ArcSwapOption};
use dashmap::DashMap;
use rtc::media_stream::MediaStreamTrack;
use rtc::rtp::Packet;
use rtc::rtp_transceiver::RTCRtpSenderId;
use rtc::rtp_transceiver::rtp_sender::{
    RTCRtpCodec, RTCRtpCodingParameters, RTCRtpEncodingParameters, RtpCodecKind,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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
}

pub struct RecordingHandle {
    pub tx: mpsc::Sender<Packet>,
}

pub struct PublishedTrack {
    pub publisher: Uuid,
    pub kind: RtpCodecKind,
    pub codec: RTCRtpCodec,
    pub outputs: ArcSwap<Vec<Output>>,
    pub active: AtomicBool,
    recorder: ArcSwapOption<RecordingHandle>,
    has_keyframe: AtomicBool,
    pub simulcast_layers: DashMap<String, u32>,
}

impl PublishedTrack {
    pub fn new(publisher: Uuid, kind: RtpCodecKind, codec: RTCRtpCodec) -> PublishedTrack {
        Self {
            publisher,
            kind,
            codec,
            outputs: ArcSwap::from_pointee(Vec::new()),
            // A track only reaches this state after its first RTP packet arrived,
            // so the publisher is considered active until proven otherwise.
            active: AtomicBool::new(true),
            recorder: ArcSwapOption::empty(),
            has_keyframe: AtomicBool::new(false),
            simulcast_layers: DashMap::new(),
        }
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
                codec: self.codec.clone(),
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

        for o in self.outputs.load_full().iter() {
            match o.tx.try_send(pkt.clone()) {
                Ok(()) => {}
                Err(_) => warn!("forward {} -> {}: fanout queue full", self.publisher, o.sub),
            }
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
