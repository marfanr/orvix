use std::sync::Arc;

use tracing::info;
use uuid::Uuid;

use crate::sfu::{room::RoomSfu, room_peer::RoomPeer};

pub fn join_to_room(room: Arc<RoomSfu>) {
    // info!("invited AI into");

    let (tx, _rx) = tokio::sync::mpsc::channel(1);
    let conn_id = Uuid::now_v7();
    let ai_peer = RoomPeer::new(
        None,
        conn_id,
        Arc::from("test"),
        Arc::new(crate::peer::PeerCommandInstance { tx }),
        None,
        Arc::clone(&room),
        true,
    );


}
