use std::rc::Rc;
use uuid::Uuid;

use crate::peer::mux::Mux;

pub(super) struct PeerDriver {
    pub(super) conn_id: Uuid,
    pub(super) mux: Rc<Mux>,
}

impl Drop for PeerDriver {
    fn drop(&mut self) {
        self.mux.unregister(self.conn_id);
    }
}


impl PeerDriver {
    pub(super) async fn run() {

    }
 }
