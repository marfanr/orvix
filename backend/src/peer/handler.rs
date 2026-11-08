use crate::peer::driver::PeerDriver;

impl PeerDriver {
    /// events and reads can themselves produce more events/writes, so iterate
    /// until quiescent and flush the writes last
    pub(super) async fn drain_protocol(&mut self) {
        loop {
            let mut progressed = false;

            // while let Some(event) = self.pc.poll_event() {
            //     progressed = true;
            // }

            // while let Some(TaggedRTCMessage { now, message }) = self.pc.poll_read() {
            //     progressed = true;
            //     self.handle_inbound_message(now, message).await;
            // }

            if !progressed {
                break;
            }
        }
    }
}