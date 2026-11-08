use std::net::IpAddr;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::peer::constant::PEER_QUEUE;

pub mod cmd;
mod constant;
mod driver;
mod mux;
mod stun;
mod handler;
pub mod worker;

pub fn create_peer(conn_id: Uuid, announce_ip: IpAddr) -> cmd::PeerCommandInstance {
    let (tx, rx) = mpsc::channel(PEER_QUEUE);

    let w = worker::worker_pool();
    let room_worker = {
        let (lo, hi) = conn_id.as_u64_pair();
        ((hi ^ lo) % w.len() as u64) as usize
    };

    _ = w
        .get(room_worker)
        .expect("cant get worker")
        .send(worker::WorkerMessage::CreatePeer {
            announce_ip,
            rx,
            conn_id,
        });

    cmd::PeerCommandInstance { tx }
}
