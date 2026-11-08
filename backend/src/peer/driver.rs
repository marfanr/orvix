use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
use std::time::Instant;
use futures_util::stream::{SelectAll, Stream, StreamExt};
use rtc::data_channel::RTCDataChannelId;
use rtc::peer_connection::RTCPeerConnection;
use rtc::rtp;
use rtc::rtp_transceiver::RTCRtpSenderId;
use rtc::sansio::Protocol;
use rtc::shared::{TaggedBytesMut, TransportContext, TransportProtocol};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::{
    peer::{
        cmd::PeerCommand,
        constant::{CMD_BATCH, DEFAULT_TIMEOUT, MEDIA_BATCH},
        inbound::InboundTrack,
        mux::{Mux, MuxInbound},
    },
    utils,
};

pub struct AttachedOutput {
    pub sender_id: RTCRtpSenderId,
    pub tx: mpsc::Sender<rtp::Packet>,
}

/// Outbound track record owned by this driver
pub struct OutboundTrack {
    pub(super) mime: String,
    pub(super) pt: Option<u8>,
    pub(super) ssrc: Option<u32>,
}

pub struct MediaReceiver {
    pub sender_id: RTCRtpSenderId,
    pub rx: mpsc::Receiver<rtp::Packet>,
}

impl Stream for MediaReceiver {
    type Item = (RTCRtpSenderId, rtp::Packet);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(item)) => Poll::Ready(Some((self.sender_id, item))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub(super) struct PeerDriver {
    pub(super) conn_id: Uuid,
    pub(super) mux: Rc<Mux>,
    pub(super) local_addr: SocketAddr,
    pub(super) net_rx: mpsc::Receiver<MuxInbound>,
    pub(super) cmd_rx: mpsc::Receiver<PeerCommand>,
    pub(super) publisher_pc: RTCPeerConnection,
    pub(super) subscriber_pc: RTCPeerConnection,
    pub(super) ctrl_id: RTCDataChannelId,
    pub(super) msg_id: RTCDataChannelId,
    pub(super) inbound: HashMap<String, InboundTrack>,
    pub(super) outbound: HashMap<RTCRtpSenderId, OutboundTrack>,
    pub(super) media_rx: SelectAll<MediaReceiver>,
    pub(super) offer_outstanding: bool,
    pub(super) pending_renegotiation: bool,
    pub(super) closing: bool,
}

impl Drop for PeerDriver {
    fn drop(&mut self) {
        self.mux.unregister(self.conn_id);
    }
}

impl PeerDriver {
    /// run peer driver event loop
    pub async fn run(mut self) {
        info!("running peer driver for conn {} on {}", self.conn_id, self.local_addr);

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
            if self.closing {
                break;
            }

            self.drain_media().await;

            let now = Instant::now();
            let mut deadline = self
                .publisher_pc
                .poll_timeout()
                .unwrap_or(now + DEFAULT_TIMEOUT);
            if let Some(sub_deadline) = self.subscriber_pc.poll_timeout() {
                deadline = deadline.min(sub_deadline);
            }
            if let Some(monitor) = self.monitor_deadline(now) {
                deadline = deadline.min(monitor);
            }
            let delay = deadline.saturating_duration_since(now);

            tokio::select! {
                biased;

                cmd = self.cmd_rx.recv() => {
                    match cmd {
                        Some(cmd) => {
                            if !self.parse_cmd(cmd).await {
                                break 'a;
                            }
                        }
                        None => break 'a,
                    }
                }

                net = self.net_rx.recv() => {
                    match net {
                        Some((peer_addr, data)) => {
                            let read_pub = TaggedBytesMut {
                                now: Instant::now(),
                                transport: TransportContext {
                                    local_addr: self.local_addr,
                                    peer_addr,
                                    ecn: None,
                                    transport_protocol: TransportProtocol::UDP,
                                },
                                message: data.clone(),
                            };
                            if let Err(e) = self.publisher_pc.handle_read(read_pub) {
                                debug!("conn {}: publisher handle_read: {e}", self.conn_id);
                            }

                            let read_sub = TaggedBytesMut {
                                now: Instant::now(),
                                transport: TransportContext {
                                    local_addr: self.local_addr,
                                    peer_addr,
                                    ecn: None,
                                    transport_protocol: TransportProtocol::UDP,
                                },
                                message: data,
                            };
                            if let Err(e) = self.subscriber_pc.handle_read(read_sub) {
                                debug!("conn {}: subscriber handle_read: {e}", self.conn_id);
                            }
                        }
                        None => break 'a,
                    }
                }

                Some((sender_id, pkt)) = self.media_rx.next() => {
                    self.write_to_sender(sender_id, pkt);
                }

                _ = tokio::time::sleep(delay) => {
                    let now = Instant::now();
                    if let Err(e) = self.publisher_pc.handle_timeout(now) {
                        debug!("conn {}: publisher handle_timeout: {e}", self.conn_id);
                    }
                    if let Err(e) = self.subscriber_pc.handle_timeout(now) {
                        debug!("conn {}: subscriber handle_timeout: {e}", self.conn_id);
                    }
                    self.monitor_tick(now).await;
                }
            }
        }

        self.shutdown().await;
        info!("peer driver stopped for conn {}", self.conn_id);
    }

    async fn shutdown(&mut self) {
        let _ = self.publisher_pc.close();
        let _ = self.subscriber_pc.close();
        self.flush_writes().await;
        self.inbound.clear();
    }

    pub(super) fn bind_ufrag_from(&self, sdp: &str) {
        if let Some(u) = utils::ice_ufrag(sdp) {
            self.mux.bind_ufrag(self.conn_id, u.to_owned());
        }
    }

    pub(super) fn reset_outbound_cache(&mut self) {
        for leg in self.outbound.values_mut() {
            leg.pt = None;
            leg.ssrc = None;
        }
    }

    pub(super) async fn flush_writes(&mut self) {
        while let Some(msg) = self.publisher_pc.poll_write() {
            if let Err(e) = self
                .mux
                .socket
                .send_to(&msg.message, msg.transport.peer_addr)
                .await
            {
                warn!("socket write (publisher) to {}: {e}", msg.transport.peer_addr);
            }
        }
        while let Some(msg) = self.subscriber_pc.poll_write() {
            if let Err(e) = self
                .mux
                .socket
                .send_to(&msg.message, msg.transport.peer_addr)
                .await
            {
                warn!("socket write (subscriber) to {}: {e}", msg.transport.peer_addr);
            }
        }
    }

    async fn drain_media(&mut self) {
        let mut wrote = false;
        for _ in 0..MEDIA_BATCH {
            tokio::select! {
                biased;
                Some((sender_id, pkt)) = self.media_rx.next() => {
                    self.write_to_sender(sender_id, pkt);
                    wrote = true;
                }
                _ = std::future::ready(()) => break,
            }
        }
        if wrote {
            self.flush_writes().await;
        }
    }

    fn write_to_sender(&mut self, sender_id: RTCRtpSenderId, mut pkt: rtp::Packet) {
        let Some(mut sender) = self.subscriber_pc.rtp_sender(sender_id) else {
            return;
        };
        let Some(leg) = self.outbound.get_mut(&sender_id) else {
            return;
        };

        if leg.pt.is_none() {
            leg.pt = sender
                .get_parameters()
                .rtp_parameters
                .codecs
                .iter()
                .find(|c| c.rtp_codec.mime_type.eq_ignore_ascii_case(&leg.mime))
                .map(|c| c.payload_type);
        }
        if let Some(pt) = leg.pt {
            pkt.header.payload_type = pt;
        }

        if leg.ssrc.is_none() {
            leg.ssrc = sender.track().ssrcs().last();
        }
        if let Some(ssrc) = leg.ssrc {
            pkt.header.ssrc = ssrc;
        }

        pkt.header.extensions.clear();
        pkt.header.extension = false;

        if let Err(e) = sender.write_rtp(Instant::now(), pkt) {
            if !matches!(e, rtc::shared::error::Error::ErrClosedPipe) {
                debug!("conn {}: rtp write: {e}", self.conn_id);
            }
        }
    }
}
