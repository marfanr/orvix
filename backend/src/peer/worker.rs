use std::{net::IpAddr, ops::Add, sync::OnceLock, thread};

use tokio::sync::mpsc;
use uuid::Uuid;

use crate::peer::{
    cmd::{PeerCommand, handle_msg},
    mux::init_mux,
};

pub enum WorkerMessage {
    CreatePeer {
        announce_ip: IpAddr,
        rx: mpsc::Receiver<PeerCommand>,
        conn_id: Uuid,
    },
}

pub struct PeerWorker {
    pub send: mpsc::UnboundedSender<WorkerMessage>,
}

pub fn worker_pool() -> &'static [mpsc::UnboundedSender<WorkerMessage>] {
    static POOL: OnceLock<Vec<mpsc::UnboundedSender<WorkerMessage>>> = OnceLock::new();
    POOL.get_or_init(|| {
        let n = std::thread::available_parallelism().map_or(1, |n| n.get());
        (0..n)
            .map(|i| {
                let base_port = 4718;
                let (tx, mut rx) = mpsc::unbounded_channel::<WorkerMessage>();
                // spawn worker thread
                thread::Builder::new()
                    .name(format!("peer-worker-{i}"))
                    .spawn(move || {
                        let rt = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .expect("worker cant run");

                        let local = tokio::task::LocalSet::new();
                        local.block_on(&rt, async move {
                            let port = base_port.add(i) as u16;
                            init_mux(port).await;

                            while let Some(job) = rx.recv().await {
                                handle_msg(&job).await;
                            }
                        })
                    })
                    .expect("failed spawn worker thread");
                tx
            })
            .collect()
    })
}
