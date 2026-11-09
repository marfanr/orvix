use crate::peer::constant::{FORWARD_RID, IDLE_TIMEOUT, PLI_INTERVAL};
use crate::peer::driver::PeerDriver;
use rtc::rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication;
use rtc::rtp_transceiver::rtp_sender::RtpCodecKind;
use rtc::rtp_transceiver::{RTCRtpReceiverId, RtpStreamId};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, warn};

/// which leg owns the receiver: `false` = publisher leg, `true` = subscriber leg
/// (client media arrives on the subscriber leg, see `ws.rs::handle_offer`)
pub struct InboundTrack {
    pub(super) receiver_id: RTCRtpReceiverId,
    pub(super) subscriber: bool,
    pub(super) kind: RtpCodecKind,
    pub(super) layers: HashMap<u32, String>,
    pub(super) is_video: bool,
    pub(super) last_rtp: Instant,
    pub(super) last_pli: Option<Instant>,
    pub(super) pli_until: Instant,
    pub(super) paused: bool,
}

impl InboundTrack {
    pub(super) fn pli_wanted(&self, now: Instant) -> bool {
        self.is_video && !self.paused && self.pli_until > now
    }

    pub(super) fn next_pli_at(&self, now: Instant) -> Instant {
        self.last_pli.map_or(now, |last| last + PLI_INTERVAL)
    }
}

impl PeerDriver {
    pub(super) async fn handle_inbound_track(
        &mut self,
        subscriber: bool,
        receiver_id: RTCRtpReceiverId,
        track_id: String,
        ssrc: u32,
        rid: Option<RtpStreamId>,
    ) {
        let (kind, codec) = {
            let receiver = if subscriber {
                self.subscriber_pc.rtp_receiver(receiver_id)
            } else {
                self.publisher_pc.rtp_receiver(receiver_id)
            };
            let Some(receiver) = receiver else {
                warn!("conn {}: track open without receiver", self.conn_id);
                return;
            };
            let track = receiver.track().clone();
            let kind = track.kind();
            let Some(codec) = track.codec(ssrc).cloned() else {
                warn!("conn {}: no codec for ssrc {ssrc} ", self.conn_id);
                return;
            };

            debug!(
                "conn {}: inbound track kind={} ssrc={ssrc} codec={}",
                self.conn_id, kind, codec.mime_type
            );
            (kind, codec)
        };
        let is_video = kind == RtpCodecKind::Video;
        let rid_ = rid.unwrap_or_else(|| FORWARD_RID.to_owned());

        let now = Instant::now();

        if let Some(state) = self.inbound.get_mut(&track_id) {
            state.last_rtp = now;
            state.layers.insert(ssrc, rid_.clone());
            debug!(
                "conn {}: inbound layer registered ssrc={ssrc} rid={rid_} ({} layer(s))",
                self.conn_id,
                state.layers.len()
            );
        } else {
            self.inbound.insert(
                track_id,
                InboundTrack {
                    receiver_id,
                    subscriber,
                    kind,
                    layers: HashMap::from([(ssrc, rid_.clone())]),
                    is_video,
                    last_rtp: now,
                    last_pli: None,
                    pli_until: now,
                    paused: true,
                },
            );
        }

        // publish into the room: creates the PublishedTrack on the first layer,
        // registers simulcast layers on subsequent ones, and attaches the track
        // to every other subscriber (which renegotiates over there)
        let published = self
            .room
            .publish_track(self.conn_id, kind, codec, rid_.clone(), ssrc)
            .await;
        debug!(
            "conn {}: published rid={rid_} ssrc={ssrc} ({} output(s))",
            self.conn_id,
            published.outputs.load().len()
        );
    }

    pub(super) fn monitor_deadline(&self, now: Instant) -> Option<Instant> {
        self.inbound
            .values()
            .filter(|t| !t.paused)
            .flat_map(|t| {
                let idle = Some(t.last_rtp + IDLE_TIMEOUT);
                let pli = t.pli_wanted(now).then(|| t.next_pli_at(now));
                [idle, pli]
            })
            .flatten()
            .min()
    }

    pub(super) async fn monitor_tick(&mut self, now: Instant) {
        for state in self.inbound.values_mut() {
            if !state.paused && state.last_rtp + IDLE_TIMEOUT <= now {
                state.paused = true;
                let last_rtp_age = now.duration_since(state.last_rtp);
                info!(
                    "conn {}: track idle, paused (last rtp {}ms ago)",
                    self.conn_id,
                    last_rtp_age.as_millis()
                );

                // mirror at room level so other peers get the muted track_state
                let kind = state.kind;
                let published = self
                    .room
                    .tracks
                    .get(&self.conn_id)
                    .map(|ts| ts.iter().find(|t| t.kind == kind).map(Arc::clone))
                    .unwrap_or_default();
                if let Some(published) = published {
                    self.room.set_track_active(&published, false).await;
                }
            }

            if state.pli_wanted(now) && state.next_pli_at(now) <= now {
                state.last_pli = Some(now);
                let receiver = if state.subscriber {
                    self.subscriber_pc.rtp_receiver(state.receiver_id)
                } else {
                    self.publisher_pc.rtp_receiver(state.receiver_id)
                };
                if let Some(mut receiver) = receiver {
                    let plis: Vec<Box<dyn rtc::rtcp::Packet>> = state
                        .layers
                        .keys()
                        .map(|&ssrc| {
                            Box::new(PictureLossIndication {
                                sender_ssrc: 0,
                                media_ssrc: ssrc,
                            }) as Box<dyn rtc::rtcp::Packet>
                        })
                        .collect();
                    let ssrcs: Vec<u32> = state.layers.keys().copied().collect();
                    if let Err(e) = receiver.write_rtcp(now, plis) {
                        warn!("conn {}: failed to send PLI: {e}", self.conn_id);
                    } else {
                        debug!("conn {}: PLI sent to {ssrcs:?}", self.conn_id);
                    }
                }
            }
        }
    }
}
