use crate::sfu::record::start_record_bot;
use crate::sfu::room::RoomSfu;
use chrono::{DateTime, TimeZone, Utc};
use rtc::data_channel::RTCDataChannelMessage;
use sonic_rs::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug)]
pub(super) struct ChatMessageInbound {
    pub(super) msg: String,
    pub(super) target: String,
    pub(super) time: u64,
}

#[derive(Deserialize, Serialize, Debug)]
pub(super) struct ChatMessageOutbound {
    pub(super) msg: String,
    pub(super) from: String,
    pub(super) time: u64,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ControlMessage {
    Record,
}

pub async fn handle_data_channel(
    conn_id: Uuid,
    msg: RTCDataChannelMessage,
    label: Arc<str>,
    room: Arc<RoomSfu>,
) {
    let json = sonic_rs::from_slice::<ControlMessage>(&msg.data);
    match json {
        Ok(ControlMessage::Record) => {
            info!("Received data {label}");
            start_record_bot(room.clone()).await;
        }
        Err(_) => {}
    };
}
