// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

use crate::peer::PeerCommandInstance;
use crate::sfu::room::RoomSfu;
use axum::extract::ws::Message;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct RoomPeer {
    pub user_id: Option<Uuid>,
    pub username: Arc<str>,
    pub ws_tx: Option<mpsc::Sender<Message>>,
    pub peer: Arc<PeerCommandInstance>,
}

impl RoomPeer {
    pub async fn new(
        user_id: Option<Uuid>,
        _conn_id: Uuid,
        username: Arc<str>,
        peer: Arc<PeerCommandInstance>,
        ws_tx: Option<mpsc::Sender<Message>>,
        _room: Arc<RoomSfu>,
    ) -> Result<Arc<Self>, crate::peer::error::PeerError> {
        Ok(Arc::new(RoomPeer {
            user_id,
            username,
            ws_tx,
            peer,
        }))
    }

    pub(super) async fn send_control<T>(&self, msg: &T) -> Result<(), ()>
    where
        T: sonic_rs::Serialize + ?Sized,
    {
        let json = sonic_rs::to_string(msg).map_err(|_| ())?;
        self.peer.send_control(json).await;
        Ok(())
    }

    pub(super) async fn send_chat<T>(&self, msg: &T) -> Result<(), ()>
    where
        T: sonic_rs::Serialize + ?Sized,
    {
        let json = sonic_rs::to_string(msg).map_err(|_| ())?;
        self.peer.send_chat(json).await;
        Ok(())
    }
}
