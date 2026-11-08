use std::net::IpAddr;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::peer::constant::PEER_QUEUE;

mod cmd;
mod constant;
mod driver;
mod mux;
mod stun;
pub mod worker;

pub fn create_peer(conn_id: Uuid, announce_ip: IpAddr) -> cmd::PeerCommandInstance {
    let (tx, rx) = mpsc::channel(PEER_QUEUE);

    let w = worker::worker_pool();
    _ = w
        .get(0)
        .expect("cant get worker")
        .send(worker::WorkerMessage::CreatePeer {
            announce_ip,
            rx,
            conn_id,
        });

    cmd::PeerCommandInstance { tx }
}
