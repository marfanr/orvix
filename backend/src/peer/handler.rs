use crate::peer::driver::PeerDriver;
use rtc::peer_connection::event::{RTCPeerConnectionEvent, RTCTrackEvent};
use rtc::peer_connection::message::TaggedRTCMessage;
use rtc::peer_connection::state::RTCPeerConnectionState;
use rtc::sansio::Protocol;
use tracing::{debug, info, warn};

impl PeerDriver {
    /// events and reads can themselves produce more events/writes, so iterate
    /// until quiescent and flush the writes last
    pub(super) async fn drain_protocol(&mut self) {
        loop {
            let mut progressed = false;

            // 1. Drain publisher_pc events
            while let Some(event) = self.publisher_pc.poll_event() {
                progressed = true;
                match event {
                    RTCPeerConnectionEvent::OnIceCandidateEvent(ev) => {
                        let Ok(_json) = ev.candidate.to_json() else {
                            continue;
                        };
                        debug!("conn {}: publisher ice candidate generated", self.conn_id);
                    }
                    RTCPeerConnectionEvent::OnIceCandidateErrorEvent(e) => {
                        warn!("conn {}: publisher ice candidate error: {:?}", self.conn_id, e);
                    }
                    RTCPeerConnectionEvent::OnIceConnectionStateChangeEvent(state) => {
                        info!("conn {}: publisher ice state change {}", self.conn_id, state);
                    }
                    RTCPeerConnectionEvent::OnConnectionStateChangeEvent(state) => {
                        info!("conn {}: publisher connection state {}", self.conn_id, state);
                        if state == RTCPeerConnectionState::Failed
                            || state == RTCPeerConnectionState::Closed
                        {
                            self.closing = true;
                        }
                    }
                    RTCPeerConnectionEvent::OnTrack(RTCTrackEvent::OnOpen(init)) => {
                        debug!(
                            "conn {}: publisher track open rid={}",
                            self.conn_id,
                            init.rid.clone().unwrap_or_default()
                        );
                        self.handle_inbound_track(
                            init.receiver_id,
                            init.track_id,
                            init.ssrc,
                            init.rid,
                        )
                        .await;
                    }
                    RTCPeerConnectionEvent::OnTrack(RTCTrackEvent::OnClose(track_id)) => {
                        if self.inbound.remove(&track_id).is_some() {
                            info!("conn {}: inbound track closed", self.conn_id);
                        }
                    }
                    _ => {}
                }
            }

            // 2. Drain subscriber_pc events
            while let Some(event) = self.subscriber_pc.poll_event() {
                progressed = true;
                match event {
                    RTCPeerConnectionEvent::OnIceCandidateEvent(ev) => {
                        let Ok(_json) = ev.candidate.to_json() else {
                            continue;
                        };
                        debug!("conn {}: subscriber ice candidate generated", self.conn_id);
                    }
                    RTCPeerConnectionEvent::OnIceCandidateErrorEvent(e) => {
                        warn!("conn {}: subscriber ice candidate error: {:?}", self.conn_id, e);
                    }
                    RTCPeerConnectionEvent::OnIceConnectionStateChangeEvent(state) => {
                        info!("conn {}: subscriber ice state change {}", self.conn_id, state);
                    }
                    RTCPeerConnectionEvent::OnConnectionStateChangeEvent(state) => {
                        info!("conn {}: subscriber connection state {}", self.conn_id, state);
                        if state == RTCPeerConnectionState::Failed
                            || state == RTCPeerConnectionState::Closed
                        {
                            self.closing = true;
                        }
                    }
                    RTCPeerConnectionEvent::OnNegotiationNeededEvent => {
                        if self.offer_outstanding {
                            self.pending_renegotiation = true;
                        } else {
                            self.send_offer_to_client().await;
                        }
                    }
                    _ => {}
                }
            }

            // 3. Drain publisher_pc reads (RTP, RTCP, DataChannels)
            while let Some(TaggedRTCMessage { now, message }) = self.publisher_pc.poll_read() {
                progressed = true;
                self.handle_inbound_message(now, message).await;
            }

            // 4. Drain subscriber_pc reads
            while let Some(TaggedRTCMessage { now, message }) = self.subscriber_pc.poll_read() {
                progressed = true;
                self.handle_inbound_message(now, message).await;
            }

            if !progressed {
                break;
            }
        }

        self.flush_writes().await;
    }

    pub(super) async fn send_offer_to_client(&mut self) {
        match self.subscriber_pc.create_offer(None) {
            Ok(offer) => {
                if let Err(e) = self.subscriber_pc.set_local_description(std::time::Instant::now(), offer) {
                    warn!("conn {}: set_local_description: {e}", self.conn_id);
                } else if let Some(desc) = self.subscriber_pc.local_description() {
                    self.offer_outstanding = true;
                    info!("conn {}: renegotiation offer generated", self.conn_id);
                    self.bind_ufrag_from(&desc.sdp);
                }
            }
            Err(e) => warn!("conn {}: subscriber create_offer: {e}", self.conn_id),
        }
    }
}
