use std::{error::Error, sync::Arc, time::Duration};

use opus::{Application, Channels, Encoder};
use rtc::rtp_transceiver::rtp_sender::{RTCRtpCodec, RtpCodecKind};
use tokio::time::{Instant, MissedTickBehavior, interval};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    ai::tts::test_onnx,
    api::ws::sync_room_member,
    sfu::{room::RoomSfu, room_peer::RoomPeer, track::PublishedTrack},
};

pub struct Assistant {}

pub async fn join_to_room(room: Arc<RoomSfu>) -> Result<(), Box<dyn Error>> {
    let (tx, _rx) = tokio::sync::mpsc::channel(1);
    let conn_id = Uuid::now_v7();
    let ai_peer = RoomPeer::new(
        None,
        conn_id,
        Arc::from("AI"),
        Arc::new(crate::peer::PeerCommandInstance { tx }),
        None,
        Arc::clone(&room),
        true,
    )
    .await;

    let Ok(peer) = ai_peer else {
        return Ok(());
    };

    room.add_peer(conn_id, peer);

    // TODO: broadcast ws
    for m in room.peers.iter() {
        let Some(ws) = m.ws_tx.clone() else { continue };
        _ = sync_room_member(&ws, &room, &m.key())
            .await
            .map_err(|e| warn!("sync room member: {e}"));
    }

    let codec = RTCRtpCodec {
        mime_type: "audio/opus".to_owned(),
        clock_rate: 8000,
        channels: 1,
        ..Default::default()
    };

    let track = room
        .publish_track(
            conn_id,
            RtpCodecKind::Audio,
            codec,
            "q".to_owned(),
            0,
            format!("ai-{conn_id}"),
        )
        .await;
    room.set_track_active(&track, true).await;

    let packets = match tokio::task::spawn_blocking(test_onnx).await {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => {
            tracing::warn!("tts gagal: {e}");
            return Ok(());
        }
        Err(e) => {
            tracing::warn!("tts task panic: {e}");
            return Ok(());
        }
    };

    let mut tick = interval(Duration::from_millis(20));
    tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut seq = 0u16;
    let mut ts = 0u32;

    for encoded in packets {
        tick.tick().await;

        let pkt = rtc::rtp::Packet {
            header: rtc::rtp::header::Header {
                version: 2,
                sequence_number: seq,
                timestamp: ts,
                marker: false,
                ..Default::default()
            },
            payload: encoded.into(),
        };

        for o in track.outputs.load_full().iter() {
            let _ = o.tx.try_send(pkt.clone());
        }

        seq = seq.wrapping_add(1);
        ts = ts.wrapping_add(960);
    }

    // test opus
    // const SR: u32 = 48_000;
    // const FREQ: f32 = 440.0;
    // const AMP: f32 = 0.25;

    // let mut encoder = Encoder::new(SR, Channels::Mono, Application::Voip)?;

    // encoder.set_bitrate(opus::Bitrate::Bits(32_000))?;

    // let mut phase = 0.0f32;
    // let mut seq = 0u16;
    // let mut ts = 0u32;

    // let step = std::f32::consts::TAU * FREQ / SR as f32;
    // let start = Instant::now();

    // loop {
    //     // 20ms @ 48kHz = 960 samples
    //     let mut frame = [0i16; 960];

    //     for s in frame.iter_mut() {
    //         *s = (phase.sin() * AMP * i16::MAX as f32) as i16;

    //         phase += step;

    //         if phase >= std::f32::consts::TAU {
    //             phase -= std::f32::consts::TAU;
    //         }
    //     }

    //     // Encode PCM -> Opus
    //     let mut encoded = [0u8; 1275];

    //     let len = encoder.encode(&frame, &mut encoded)?;

    //     let pkt = rtc::rtp::Packet {
    //         header: rtc::rtp::header::Header {
    //             version: 2,
    //             sequence_number: seq,
    //             timestamp: ts,
    //             marker: false,
    //             ..Default::default()
    //         },
    //         payload: encoded[..len].to_vec().into(),
    //     };

    //     for o in track.outputs.load_full().iter() {
    //         let _ = o.tx.try_send(pkt.clone());
    //     }

    //     seq = seq.wrapping_add(1);

    //     // Opus RTP clock rate = 48000 Hz
    //     // 20ms = 960 timestamp units
    //     ts = ts.wrapping_add(960);

    //     tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    //     if start.elapsed().as_millis() > 400 {
    //         break;
    //     }
    // }
    Ok(())
}
