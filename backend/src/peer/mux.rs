use bytes::BytesMut;
use std::{cell::RefCell, collections::HashMap, net::SocketAddr, rc::Rc, time::Duration};
use tokio::{net::UdpSocket, sync::mpsc};
use uuid::Uuid;

use crate::{peer::{constant::PEER_QUEUE, stun::gather_srflx}, utils::stun_local_ufrag};

pub type MuxInbound = (SocketAddr, BytesMut);

pub struct Mux {
    pub socket: UdpSocket,
    pub port: u16,
    peers: RefCell<HashMap<Uuid, mpsc::Sender<MuxInbound>>>,
    ufrags: RefCell<HashMap<String, Uuid>>,
    pub srflx_addr: Option<SocketAddr>,
    addrs: RefCell<HashMap<SocketAddr, Uuid>>,
}

thread_local! {
    static MUX: RefCell<Option<Rc<Mux>>> = RefCell::new(None);
}

pub async fn init_mux(port: u16) {
    let Ok(socket) = UdpSocket::bind(("0.0.0.0", port)).await else {
        return;
    };
    let slfx_addr = gather_srflx(&socket, "stun.l.google.com:19302").await.ok();

    let mux = Rc::new(Mux {
        socket,
        port,
        peers: Default::default(),
        ufrags: Default::default(),
        addrs: Default::default(),
        srflx_addr: slfx_addr,
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
            let (n, from) = match self.socket.recv_buf_from(&mut buf).await {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!("mux recv: {e}");
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    continue;
                }
            };

            let data = &buf[..n];

            let id = if let Some(ufrag) = stun_local_ufrag(data) {
                let id = self.ufrags.borrow().get(&ufrag).copied();
                if let Some(id) = id {
                    self.addrs.borrow_mut().insert(from, id);
                }
                id
            } else {
                self.addrs.borrow().get(&from).copied()
            };

            if let Some(id) = id {
                if let Some(tx) = self.peers.borrow().get(&id) {
                    let _ = tx.try_send((from, BytesMut::from(data)));
                }
            }
        }
    }

    pub fn register(&self, id: Uuid) -> mpsc::Receiver<MuxInbound> {
        let (tx, rx) = mpsc::channel(PEER_QUEUE);
        self.peers.borrow_mut().insert(id, tx);
        rx
    }

    pub fn bind_ufrag(&self, id: Uuid, ufrag: String) {
        let mut m = self.ufrags.borrow_mut();
        m.retain(|_, v| *v != id);
        m.insert(ufrag, id);
    }

    pub fn unregister(&self, id: Uuid) {
        self.peers.borrow_mut().remove(&id);
        self.ufrags.borrow_mut().retain(|_, v| *v != id);
        self.addrs.borrow_mut().retain(|_, v| *v != id);
    }
}
