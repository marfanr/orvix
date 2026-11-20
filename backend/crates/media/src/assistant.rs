// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

use std::{
    error::Error,
    io::Cursor,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};

use ai::{core::LangCore, onnx::Results, tts::speak};
use arc_swap::ArcSwapOption;
use bytes::Bytes;
use oxideav_core::RuntimeContext;
use oxideav_mkv::demux::TrackAudio;
use rtc::{
    rtp::codec::vp9::Vp9Payloader,
    rtp::packetizer::Payloader,
    rtp_transceiver::rtp_sender::{RTCRtpCodec, RtpCodecKind},
};
use tokio::{
    sync::mpsc,
    time::{Instant, MissedTickBehavior, interval, timeout, timeout_at},
};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    peer::PeerCommandInstance,
    sfu::{room::RoomSfu, room_peer::RoomPeer, track::PublishedTrack},
    sync_room_member,
};

pub struct Assistant {
    in_tx: mpsc::Sender<String>,
    room: Arc<RoomSfu>,
    running: Arc<AtomicBool>,
    lang: Arc<LangCore>,
    audio_track: ArcSwapOption<PublishedTrack>,
}

#[derive(Default)]
struct SpeakState {
    seq: u16,
    ts: u32,
    last: Option<Instant>,
}

impl Assistant {
    pub fn new(room: Arc<RoomSfu>) -> Results<Arc<Self>> {
        let (in_tx, in_rx) = mpsc::channel::<String>(1024);
        let lang = Arc::new(LangCore::new()?);
        let instance = Arc::new(Self {
            in_tx,
            room,
            running: Arc::new(AtomicBool::new(false)),
            lang,
            audio_track: ArcSwapOption::empty(),
        });

        instance.spawn(in_rx);
        info!("onstance spawned");
        Ok(instance)
    }

    fn spawn(self: &Arc<Self>, mut rx: mpsc::Receiver<String>) {
        if self
            .running
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }

        let room = Arc::clone(&self.room);
        let running = Arc::clone(&self.running);

        let (speak_tx, mut speak_rx) = mpsc::channel::<String>(16);
        let speaker = Arc::clone(self);
        tokio::spawn(async move {
            let mut speak_state = SpeakState::default();

            while let Some(text) = speak_rx.recv().await {
                if let Some(track) = speaker.audio_track.load_full() {
                    _ = try_speak(track, text, &mut speak_state).await;
                }
            }
        });

        let instance = Arc::clone(self);
        tokio::spawn(async move {
            if let Err(e) = instance.join(&room).await {
                warn!("assistant join: {e}");
            }

            const BATCH_WINDOW: Duration = Duration::from_millis(500);
            const LLM_TIMEOUT: Duration = Duration::from_secs(60);

            while let Some(first) = rx.recv().await {
                info!("msg: {first}");
                let mut input = first;

                let deadline = Instant::now() + BATCH_WINDOW;
                while let Ok(Some(msg)) = timeout_at(deadline, rx.recv()).await {
                    info!("msg: {msg}");
                    input.push(' ');
                    input.push_str(&msg);
                }

                let input = input.trim();
                if input.is_empty() || (input.starts_with('[') && input.ends_with(']')) {
                    continue;
                }

                match timeout(LLM_TIMEOUT, instance.lang.call(input)).await {
                    Ok(Ok(res)) => {
                        info!("res: {res}");
                        if speak_tx.try_send(res).is_err() {
                            warn!("speak queue unavailable");
                        }
                    }
                    Ok(Err(err)) => warn!("err: {}", err.0),
                    Err(_) => warn!("timeout: {input}"),
                }
            }

            running.store(false, Ordering::Release);
        });
    }

    async fn join(&self, room: &Arc<RoomSfu>) -> Result<(), Box<dyn Error + Send + Sync>> {
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let conn_id = Uuid::now_v7();

        let dummy_peer = Arc::new(PeerCommandInstance { tx });

        let peer = RoomPeer::new(
            None,
            conn_id,
            Arc::from("AI"),
            dummy_peer,
            None,
            Arc::clone(room),
            true,
        )
        .await
        .map_err(|_| "failed to create AI peer")?;

        room.add_peer(conn_id, peer);

        // TODO: broadcast ws
        let members: Vec<_> = room
            .peers
            .iter()
            .filter_map(|m| Some((m.ws_tx.clone()?, m.key().clone())))
            .collect();

        for (ws, key) in members {
            _ = sync_room_member(&ws, room, &key)
                .await
                .map_err(|e| warn!("sync room member: {e}"));
        }

        let audio_codec = RTCRtpCodec {
            mime_type: "audio/opus".to_owned(),
            clock_rate: 48_000,
            channels: 2,
            ..Default::default()
        };

        let audio_track = room
            .publish_track(
                conn_id,
                RtpCodecKind::Audio,
                audio_codec,
                "q".to_owned(),
                0,
                format!("ai-{conn_id}"),
            )
            .await;

        // room.set_track_active(&audio_track, false).await;

        self.audio_track.store(Some(audio_track));

        // let video_codec = RTCRtpCodec {
        //     mime_type: "video/VP9".to_owned(),
        //     clock_rate: 90_000,
        //     ..Default::default()
        // };
        // let video_track = room
        //     .publish_track(
        //         conn_id,
        //         RtpCodecKind::Video,
        //         video_codec,
        //         "q".to_owned(),
        //         0,
        //         format!("ai-v-{conn_id}"),
        //     )
        //     .await;

        // room.set_track_active(track, active);

        // let track = Arc::clone(&video_track);
        // let room_ = Arc::clone(room);
        // let handle = tokio::runtime::Handle::current();

        // tokio::task::spawn_blocking(move || {
        //     if let Err(e) = handle.block_on(Self::load_media(track, room_)) {
        //         warn!("load_media: {e}");
        //     }
        // });

        Ok(())
    }

    pub async fn forward(&self, msg: String) {
        _ = self.in_tx.send(msg).await;
    }

    fn find_start_keyframe(
        ctx: &RuntimeContext,
        media_bytes: &Arc<[u8]>,
        start_ms: u64,
    ) -> Result<u64, Box<dyn Error + Send + Sync>> {
        let mut input: Box<dyn oxideav_core::ReadSeek> =
            Box::new(Cursor::new(Arc::clone(media_bytes)));
        let format = ctx.containers.probe_input(input.as_mut(), Some("webm"))?;
        let mut demuxer = ctx.containers.open_demuxer(&format, input, &ctx.codecs)?;

        let codecs: Vec<String> = demuxer
            .streams()
            .iter()
            .map(|s| s.params.codec_id.as_str().to_owned())
            .collect();

        let mut best: Option<u64> = None;

        loop {
            let pkt = match demuxer.next_packet() {
                Ok(p) => p,
                Err(oxideav_core::Error::Eof) => break,
                Err(e) => return Err(e.into()),
            };

            if pkt.flags.header || pkt.flags.discard {
                continue;
            }
            if codecs[pkt.stream_index as usize] != "vp9" {
                continue;
            }

            let pts_ms = pkt.pts.unwrap_or(0).max(0) as u64;
            if pts_ms > start_ms {
                break;
            }
            if pkt.flags.keyframe {
                best = Some(pts_ms);
            }
        }

        // Kalau tidak ada keyframe <= start, biarkan wait_key yang cari keyframe berikutnya.
        Ok(best.unwrap_or(start_ms))
    }

    async fn load_media(
        track: Arc<PublishedTrack>,
        room: Arc<RoomSfu>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        const MTU: usize = 1200;

        let media_path = common::utils::project_root().join("media/char.webm");

        let mut ctx = RuntimeContext::new();
        oxideav_mkv::register(&mut ctx);

        let mut payloader = Vp9Payloader::default();
        let start = Instant::now();
        let mut seq = 0u16;
        let mut base_ms = 0u64;

        let media_bytes: Arc<[u8]> = std::fs::read(&media_path)?.into();

        let loop_start = Arc::new(AtomicU64::new(0));
        let loop_end = Arc::new(AtomicU64::new(5000));

        loop {
            let start_ms = loop_start.load(Ordering::Relaxed);
            let end_ms = loop_end.load(Ordering::Relaxed);

            let eff_start_ms = Self::find_start_keyframe(&ctx, &media_bytes, start_ms)?;

            let mut input: Box<dyn oxideav_core::ReadSeek> =
                Box::new(Cursor::new(Arc::clone(&media_bytes)));
            let format = ctx.containers.probe_input(input.as_mut(), Some("webm"))?;
            let mut demuxer = ctx.containers.open_demuxer(&format, input, &ctx.codecs)?;

            let codecs: Vec<String> = demuxer
                .streams()
                .iter()
                .map(|s| s.params.codec_id.as_str().to_owned())
                .collect();

            let mut wait_key = true;
            let mut first_pts: Option<u64> = None;
            let mut pass_end_ms = 0u64;
            let mut range_changed = false;

            loop {
                if loop_start.load(Ordering::Relaxed) != start_ms
                    || loop_end.load(Ordering::Relaxed) != end_ms
                {
                    range_changed = true;
                    break;
                }

                let pkt = match demuxer.next_packet() {
                    Ok(p) => p,
                    Err(oxideav_core::Error::Eof) => break,
                    Err(e) => return Err(e.into()),
                };

                room.set_track_active(&track, true).await;

                if pkt.flags.header || pkt.flags.discard {
                    continue;
                }

                if codecs[pkt.stream_index as usize] != "vp9" {
                    continue;
                }

                let pts_ms = pkt.pts.unwrap_or(0).max(0) as u64;
                let dur_ms = pkt.duration.unwrap_or(33).max(0) as u64;

                // sebelum keyframe start: skip
                if pts_ms < eff_start_ms {
                    continue;
                }
                // lewat end: selesai pass
                if pts_ms >= end_ms {
                    break;
                }

                // mulai dari keyframe pertama di/setelah start efektif
                if wait_key {
                    if !pkt.flags.keyframe {
                        continue;
                    }
                    wait_key = false;
                }

                let first = *first_pts.get_or_insert(pts_ms);
                let rel_ms = pts_ms - first;
                pass_end_ms = pass_end_ms.max(rel_ms + dur_ms);

                let t_ms = base_ms + rel_ms;

                let due = start + Duration::from_millis(t_ms);
                let now = Instant::now();
                if due > now {
                    std::thread::sleep(due - now);
                }

                let ts = (t_ms * 90) as u32; // 90 kHz, wrap u32 itu normal
                let frags = payloader.payload(MTU, &Bytes::from(pkt.data))?;
                let last = frags.len().saturating_sub(1);

                for (i, payload) in frags.into_iter().enumerate() {
                    let out = rtc::rtp::Packet {
                        header: rtc::rtp::header::Header {
                            version: 2,
                            sequence_number: seq,
                            timestamp: ts,
                            marker: i == last,
                            ..Default::default()
                        },
                        payload,
                    };

                    for o in track.outputs.load_full().iter() {
                        let _ = o.tx.try_send(out.clone());
                    }

                    seq = seq.wrapping_add(1);
                }
            }

            if pass_end_ms == 0 {
                if range_changed {
                    continue;
                }
                return Err("no VP9 frames in range".into());
            }

            base_ms += pass_end_ms;
        }
    }
}

async fn try_speak(
    track: Arc<PublishedTrack>,
    text: String,
    state: &mut SpeakState,
) -> Results<()> {
    let packets = match tokio::task::spawn_blocking(|| ai::tts::speak(text)).await {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => {
            tracing::warn!("tts failed: {e}");
            return Ok(());
        }
        Err(e) => {
            tracing::warn!("tts task panic: {e}");
            return Ok(());
        }
    };

    if let Some(last) = state.last {
        let gap = (last.elapsed().as_millis() as u64 * 48).saturating_sub(960) as u32;
        state.ts = state.ts.wrapping_add(gap);
    }

    let mut tick = interval(Duration::from_millis(20));
    tick.set_missed_tick_behavior(MissedTickBehavior::Delay);

    for (i, encoded) in packets.into_iter().enumerate() {
        tick.tick().await;

        let pkt = rtc::rtp::Packet {
            header: rtc::rtp::header::Header {
                version: 2,
                sequence_number: state.seq,
                timestamp: state.ts,
                marker: i == 0,
                ..Default::default()
            },
            payload: encoded.into(),
        };

        for o in track.outputs.load_full().iter() {
            let _ = o.tx.try_send(pkt.clone());
        }

        state.seq = state.seq.wrapping_add(1);
        state.ts = state.ts.wrapping_add(960);
        state.last = Some(Instant::now());
    }

    Ok(())
}
