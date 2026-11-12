use crate::peer::driver::PeerDriver;
use axum::extract::ws;
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

            // Drain publisher_pc events
            while let Some(event) = self.publisher_pc.poll_event() {
                progressed = true;
                match event {
                    RTCPeerConnectionEvent::OnIceCandidateEvent(ev) => {
                        let Ok(json) = ev.candidate.to_json() else {
                            continue;
                        };
                        _ = self
                            .ws_sink
                            .send(ws::Message::text(
                                sonic_rs::json!({
                                    "type": "ice",
                                    "peer_type": "PUBLISHER",
                                    "candidate": &json.candidate,
                                    "sdpMid": &json.sdp_mid,
                                    "sdpMLineIndex": &json.sdp_mline_index,
                                    "usernameFragment": &json.username_fragment,
                                })
                                .to_string(),
                            ))
                            .await;
                    }
                    RTCPeerConnectionEvent::OnConnectionStateChangeEvent(state) => {
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
                    RTCPeerConnectionEvent::OnTrack(RTCTrackEvent::OnOpen(init)) => {
                        info!(
                            "conn {}: publisher track open rid={}",
                            self.conn_id,
                            init.rid.clone().unwrap_or_default()
                        );
                        self.handle_inbound_track(
                            false,
                            init.receiver_id,
                            init.track_id,
                            init.ssrc,
                            init.rid,
                        )
                        .await;
                    }
                    RTCPeerConnectionEvent::OnTrack(RTCTrackEvent::OnClose(track_id)) => {
                        if self.inbound.remove(&track_id).is_some() {
                            info!("conn {}: publisher inbound track closed", self.conn_id);
                        }
                    }
                    _ => {}
                }
            }

            // Drain subscriber_pc events
            while let Some(event) = self.subscriber_pc.poll_event() {
                progressed = true;
                match event {
                    RTCPeerConnectionEvent::OnIceCandidateEvent(ev) => {
                        let Ok(json) = ev.candidate.to_json() else {
                            continue;
                        };

                        _ = self
                            .ws_sink
                            .send(ws::Message::text(
                                sonic_rs::json!({
                                    "type": "ice",
                                    "peer_type": "SUBSCRIBER",
                                    "candidate": &json.candidate,
                                    "sdpMid": &json.sdp_mid,
                                    "sdpMLineIndex": &json.sdp_mline_index,
                                    "usernameFragment": &json.username_fragment,
                                })
                                .to_string(),
                            ))
                            .await;
                    }
                    RTCPeerConnectionEvent::OnIceCandidateErrorEvent(e) => {
                        warn!(
                            "conn {}: subscriber ice candidate error: {:?}",
                            self.conn_id, e
                        );
                    }
                    RTCPeerConnectionEvent::OnConnectionStateChangeEvent(state) => {
                        if state == RTCPeerConnectionState::Failed
                            || state == RTCPeerConnectionState::Closed
                        {
                            self.closing = true;
                        }
                    }
                    RTCPeerConnectionEvent::OnTrack(RTCTrackEvent::OnOpen(init)) => {
                        // the client's publisher media arrives on this leg
                        info!(
                            "conn {}: subscriber track open rid={}",
                            self.conn_id,
                            init.rid.clone().unwrap_or_default()
                        );
                        self.handle_inbound_track(
                            true,
                            init.receiver_id,
                            init.track_id,
                            init.ssrc,
                            init.rid,
                        )
                        .await;
                    }
                    RTCPeerConnectionEvent::OnTrack(RTCTrackEvent::OnError(err)) => {
                        tracing::error!("RTCTrackEvent::OnError {err}");
                    }
                    RTCPeerConnectionEvent::OnTrack(RTCTrackEvent::OnClose(track_id)) => {
                        info!("track close : {}", &track_id);
                        if self.inbound.remove(&track_id).is_some() {
                            info!("conn {}: subscriber inbound track closed", self.conn_id);
                        }
                    }
                    _ => {}
                }
            }

            // Drain publisher_pc reads (RTP, RTCP, DataChannels)
            while let Some(TaggedRTCMessage { now, message }) = self.publisher_pc.poll_read() {
                progressed = true;
                self.handle_inbound_message(now, message).await;
            }

            // Drain subscriber_pc reads
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
        match self.publisher_pc.create_offer(None) {
            Ok(offer) => {
                if let Err(e) = self
                    .publisher_pc
                    .set_local_description(std::time::Instant::now(), offer)
                {
                    warn!("conn {}: set_local_description: {e}", self.conn_id);
                } else if let Some(desc) = self.publisher_pc.local_description() {
                    self.offer_outstanding = true;
                    info!("conn {}: renegotiation offer generated", self.conn_id);
                    self.bind_ufrag_from(&desc.sdp, crate::peer::PeerType::PUBLISHER);

                    let msg = sonic_rs::json!({ "type": "offer", "sdp": desc.sdp }).to_string();
                    if let Err(e) = self.ws_sink.send(ws::Message::text(msg)).await {
                        warn!("conn {}: send offer to client: {e}", self.conn_id);
                    }
                }
            }
            Err(e) => warn!("conn {}: publisher create_offer: {e}", self.conn_id),
        }
    }
}
