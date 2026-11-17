// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

use std::{error::Error, sync::Arc, time::Duration};

use rtc::rtp_transceiver::rtp_sender::{RTCRtpCodec, RtpCodecKind};
use tokio::time::{MissedTickBehavior, interval};
use tracing::warn;
use uuid::Uuid;

use ai::tts::test_onnx;

use crate::{
    peer::PeerCommandInstance,
    sfu::{room::RoomSfu, room_peer::RoomPeer},
    sync_room_member,
};

pub struct Assistant {}

pub async fn join_to_room(room: Arc<RoomSfu>) -> Result<(), Box<dyn Error>> {
    let (tx, _rx) = tokio::sync::mpsc::channel(1);
    let conn_id = Uuid::now_v7();
    let ai_peer = RoomPeer::new(
        None,
        conn_id,
        Arc::from("AI"),
        Arc::new(PeerCommandInstance { tx }),
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
            tracing::warn!("tts failed: {e}");
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

    Ok(())
}
