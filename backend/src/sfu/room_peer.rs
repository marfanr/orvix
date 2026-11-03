// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

use crate::sfu::data_channel::{ChatMessageInbound, ChatMessageOutbound};
use axum::extract::ws::Message;
use futures_util::FutureExt;
use rtc::data_channel::RTCDataChannelInit;
use rtc::rtp_transceiver::rtp_sender::RtpCodecKind;
use rtc::rtp_transceiver::{RTCRtpTransceiverDirection, RTCRtpTransceiverInit};
use std::sync::Arc;
use tokio::select;
use tokio::sync::mpsc;
use uuid::Uuid;
use webrtc::data_channel::{DataChannel, DataChannelEvent};
use webrtc::peer_connection::PeerConnection;
use crate::sfu::room::RoomSfu;

pub struct RoomPeer {
    pub user_id: Option<Uuid>,
    pub username: Arc<str>,
    pub ws_tx: Option<mpsc::Sender<Message>>,
    pub pub_pc: Arc<dyn PeerConnection>,
    pub sub_pc: Arc<dyn PeerConnection>,
    ctrl: Arc<dyn DataChannel>,
    chat: Arc<dyn DataChannel>,
}

impl RoomPeer {
    pub async fn new(
        user_id: Option<Uuid>,
        conn_id: Uuid,
        username: Arc<str>,
        publisher_pc: Arc<dyn PeerConnection>,
        subscriber_pc: Arc<dyn PeerConnection>,
        ws_tx: Option<mpsc::Sender<Message>>,
        room: Arc<RoomSfu>
    ) -> Result<Arc<Self>, webrtc::error::Error> {
        let ctrl = publisher_pc.create_data_channel("control", None).await?;
        let chat_receiver = subscriber_pc
            .create_data_channel(
                "chat",
                Some(RTCDataChannelInit {
                    negotiated: Some(1),
                    ..Default::default()
                }),
            )
            .await?;

        let chat_sender = publisher_pc
            .create_data_channel(
                "chat",
                Some(RTCDataChannelInit {
                    negotiated: Some(1),
                    ..Default::default()
                }),
            )
            .await?;

        let username_ = Arc::clone(&username);
        tokio::spawn(async move {
            loop {
                select! {
                    chat_res = chat_receiver.poll().fuse() => {
                        match chat_res {
                            Some(DataChannelEvent::OnMessage(msg)) => {
                                if let Ok(json) = sonic_rs::from_slice::<ChatMessageInbound>(&msg.data) {
                                    tracing::info!("new message {}", json.msg);
                                    let reply_msg = ChatMessageOutbound {
                                        msg: json.msg,
                                        from: username_.to_string(),
                                        time: json.time,
                                    };
                                    room.broadcast_chat(conn_id, &reply_msg).await;
                                }

                            }
                            Some(_) => {}
                            _ => {}
                        }
                    }
                }
            }
        });

        subscriber_pc
            .add_transceiver_from_kind(
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
            chat: chat_sender,
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

    pub(super) async fn send_chat<T>(&self, msg: &T) -> Result<(), ()>
    where
        T: sonic_rs::Serialize + ?Sized,
    {
        let json = sonic_rs::to_string(msg).map_err(|_| ())?;
        self.chat.send_text(json.as_str()).await.map_err(|_| ())?;
        Ok(())
    }
}
