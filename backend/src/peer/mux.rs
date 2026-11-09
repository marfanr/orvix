use bytes::BytesMut;
use std::{cell::RefCell, collections::HashMap, net::SocketAddr, rc::Rc, time::Duration};
use tokio::{net::UdpSocket, sync::mpsc};
use uuid::Uuid;

use crate::{
    peer::{PeerType, constant::PEER_QUEUE, stun::gather_srflx},
    utils::stun_local_ufrag,
};

/// (remote addr, payload, leg of the conn this packet belongs to)
pub type MuxInbound = (SocketAddr, BytesMut, PeerType);

pub struct Mux {
    pub socket: UdpSocket,
    pub port: u16,
    peers: RefCell<HashMap<Uuid, mpsc::Sender<MuxInbound>>>,
    ufrags: RefCell<HashMap<String, (Uuid, PeerType)>>,
    pub srflx_addr: Option<SocketAddr>,
    addrs: RefCell<HashMap<SocketAddr, (Uuid, PeerType)>>,
}

thread_local! {
    static MUX: RefCell<Option<Rc<Mux>>> = RefCell::new(None);
}

pub async fn init_mux(port: u16) {
    let socket = match UdpSocket::bind(("0.0.0.0", port)).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("mux bind udp 0.0.0.0:{port}: {e}");
            return;
        }
    };
    let srflx_addr = gather_srflx(&socket, "stun.l.google.com:19302").await.ok();

    let mux = Rc::new(Mux {
        socket,
        port,
        peers: Default::default(),
        ufrags: Default::default(),
        addrs: Default::default(),
        srflx_addr,
    });
    MUX.with(|m| *m.borrow_mut() = Some(Rc::clone(&mux)));
    tokio::task::spawn_local(mux.run());
}

pub fn current() -> Rc<Mux> {
    MUX.with(|m| {
        m.borrow()
            .clone()
            .expect("mux not initialised on this thread")
    })
}

impl Mux {
    async fn run(self: Rc<Self>) {
        let mut buf = vec![0u8; 2048];
        loop {
            let (n, from) = match self.socket.recv_from(&mut buf).await {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("mux recv: {e}");
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    continue;
                }
            };

            let data = &buf[..n];

            let route = if let Some(ufrag) = stun_local_ufrag(data) {
                let route = self.ufrags.borrow().get(&ufrag).copied();
                match route {
                    Some(r) => {
                        self.addrs.borrow_mut().insert(from, r);
                    }
                    None => tracing::debug!("mux: stun from {from} with unknown ufrag {ufrag}"),
                }
                route
            } else {
                self.addrs.borrow().get(&from).copied()
            };

            let Some((id, leg)) = route else {
                tracing::trace!("mux: drop {n}B from {from}, no route");
                continue;
            };

            if let Some(tx) = self.peers.borrow().get(&id) {
                if tx.try_send((from, BytesMut::from(data), leg)).is_err() {
                    tracing::warn!(
                        "mux: queue full or closed for {id}, dropping packet from {from}"
                    );
                }
            }
        }
    }

    pub fn register(&self, id: Uuid) -> mpsc::Receiver<MuxInbound> {
        let (tx, rx) = mpsc::channel(PEER_QUEUE);
        self.peers.borrow_mut().insert(id, tx);
        rx
    }

    /// Binds a local ufrag to (conn, leg). Both legs of one conn each have
    /// their own ufrag; all are kept.
    pub fn bind_ufrag(&self, id: Uuid, leg: PeerType, ufrag: String) {
        self.ufrags.borrow_mut().insert(ufrag, (id, leg));
    }

    pub fn unregister(&self, id: Uuid) {
        self.peers.borrow_mut().remove(&id);
        self.ufrags.borrow_mut().retain(|_, v| v.0 != id);
        self.addrs.borrow_mut().retain(|_, v| v.0 != id);
    }
}
