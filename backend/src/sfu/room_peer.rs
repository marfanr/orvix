// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

use axum::extract::ws::Message;
use std::sync::Arc;
use rtc::rtp_transceiver::{RTCRtpTransceiverDirection, RTCRtpTransceiverInit};
use rtc::rtp_transceiver::rtp_sender::RtpCodecKind;
use tokio::sync::mpsc;
use uuid::Uuid;
use webrtc::data_channel::DataChannel;
use webrtc::peer_connection::PeerConnection;

pub struct RoomPeer {
    pub user_id: Option<Uuid>,
    pub username: Arc<str>,
    pub ws_tx: Option<mpsc::Sender<Message>>,
    pub pub_pc: Arc<dyn PeerConnection>,
    pub sub_pc: Arc<dyn PeerConnection>,
    ctrl: Arc<dyn DataChannel>,
}

impl RoomPeer {
    pub async fn new(
        user_id: Option<Uuid>,
        username: Arc<str>,
        publisher_pc: Arc<dyn PeerConnection>,
        subscriber_pc: Arc<dyn PeerConnection>,
        ws_tx: Option<mpsc::Sender<Message>>,
    ) -> Result<Arc<Self>, webrtc::error::Error> {
        let ctrl = publisher_pc.create_data_channel("control", None).await?;

        subscriber_pc.add_transceiver_from_kind(
            RtpCodecKind::Video,
            Some(RTCRtpTransceiverInit {
                direction: RTCRtpTransceiverDirection::Recvonly,
                ..Default::default()
            }),
        )
            .await
            .map_err(webrtc::error::Error::from)?;

        Ok(Arc::new(RoomPeer {
            user_id,
            username,
            ws_tx,
            pub_pc: publisher_pc,
            sub_pc: subscriber_pc,
            ctrl,
        }))
    }

    pub(super) async fn send_control<T>(&self, msg: &T) -> Result<(), ()>
    where
        T: sonic_rs::Serialize + ?Sized,
    {
        let json = sonic_rs::to_string(msg).map_err(|_| ())?;
        self.ctrl.send_text(json.as_str()).await.map_err(|_| ())?;
        Ok(())
    }
}
