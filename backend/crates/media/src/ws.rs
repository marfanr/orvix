// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

use axum::extract::ws::Message;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::sfu::room::RoomSfu;

pub async fn sync_room_member(
    sink: &mpsc::Sender<Message>,
    r: &RoomSfu,
    conn_id: &Uuid,
) -> Result<(), String> {
    let members: Vec<_> = r
        .peers
        .iter()
        .filter(|p| p.key() != conn_id && p.value().is_admitted())
        .map(|p| {
            sonic_rs::json!({
                "conn_id": p.key(),
                "username": &*p.value().username,
            })
        })
        .collect();

    sink.send(Message::text(
        sonic_rs::json!({ "type": "sync", "members": members}).to_string(),
    ))
    .await
    .map_err(|e| e.to_string())
}
