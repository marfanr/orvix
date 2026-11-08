use std::rc::Rc;
use rtc::peer_connection::RTCPeerConnection;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{peer::{
    cmd::PeerCommand,
    constant::CMD_BATCH,
    mux::{Mux, MuxInbound},
}, utils};


pub(super) struct PeerDriver {
    pub(super) conn_id: Uuid,
    pub(super) mux: Rc<Mux>,
    pub(super) net_rx: mpsc::Receiver<MuxInbound>,
    pub(super) cmd_rx: mpsc::Receiver<PeerCommand>,
    pub(super) publisher_pc: RTCPeerConnection,
    pub(super) subscriber_pc: RTCPeerConnection,
}

impl Drop for PeerDriver {
    fn drop(&mut self) {
        self.mux.unregister(self.conn_id);
    }
}

impl PeerDriver {
    /// run peer driver same at the worker core
    pub async fn run(mut self) {
        tracing::info!("running peer...");
        'a: loop {
            for _ in 0..CMD_BATCH {
                match self.cmd_rx.try_recv() {
                    Ok(cmd) => {
                        if !self.parse_cmd(cmd).await {
                            break 'a;
                        }
                    }
                    Err(mpsc::error::TryRecvError::Empty) => break,
                    Err(mpsc::error::TryRecvError::Disconnected) => break 'a,
                }
            }

            self.drain_protocol().await;
        }
    }

    pub(super) fn bind_ufrag_from(&self, sdp: &str) {
        if let Some(u) = utils::ice_ufrag(sdp) {
            self.mux.bind_ufrag(self.conn_id, u.to_owned());
        }
    }

}
