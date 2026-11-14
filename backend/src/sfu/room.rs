// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

use crate::models;
use crate::models::room::Room;
use crate::sfu::room_peer::RoomPeer;
use crate::sfu::svc::LayerSelector;
use crate::sfu::track::{Output, PublishedTrack};
use arc_swap::{ArcSwap, ArcSwapOption};
use axum::extract::ws::Message;
use dashmap::DashMap;
use rtc::rtp_transceiver::rtp_sender::{RTCRtpCodec, RtpCodecKind};
use sonic_rs::Serialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tracing::warn;
use uuid::Uuid;

pub enum RoomMemberstype {
    Waiting,
}

pub struct RoomSfu {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub alias: Arc<str>,
    pub peers: DashMap<Uuid, Arc<RoomPeer>>,
    pub tracks: DashMap<Uuid, Vec<Arc<PublishedTrack>>>,
    pub recording: AtomicBool,
    pub pending_joins: AtomicUsize,
}

pub struct JoinToken(Arc<RoomSfu>);

impl Drop for JoinToken {
    fn drop(&mut self) {
        self.0.pending_joins.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Debug, Serialize)]
pub struct TrackState {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub from: Uuid,
    pub kind: String,
    pub muted: bool,
}

impl RoomSfu {
    pub fn register_join(self: &Arc<Self>) -> JoinToken {
        self.pending_joins.fetch_add(1, Ordering::AcqRel);
        JoinToken(Arc::clone(self))
    }

    pub fn can_be_removed(&self) -> bool {
        self.peers.is_empty() && self.pending_joins.load(Ordering::Acquire) == 0
    }

    pub fn add_peer(&self, conn_id: Uuid, peer: Arc<RoomPeer>) -> bool {
        if self.peers.iter().any(|p| p.key() == &conn_id) {
            return false;
        }
        self.peers.insert(conn_id, peer);
        true
    }

    pub async fn drop_peer(&self, conn_id: Uuid) {
        self.peers.remove(&conn_id);

        if let Some((_, tracks)) = self.tracks.remove(&conn_id) {
            for track in tracks {
                track.stop_recording();
            }
        }

        // Snapshot the outputs first: holding a DashMap guard across an await
        // blocks every other task touching this map (e.g. publish_track).
        let outputs: Vec<Arc<PublishedTrack>> = self
            .tracks
            .iter()
            .map(|entry| entry.value().clone())
            .flatten()
            .collect();

        for track in outputs {
            track.drop_output(conn_id).await;
        }
    }

    pub async fn attach_existing_tracks(&self, sub_id: Uuid, peer: &RoomPeer) {
        let mut attached_any = false;
        for entry in self.tracks.iter() {
            if entry.key() == &sub_id {
                continue;
            }
            for track in entry.value().iter() {
                if self.attach(track, sub_id, peer).await {
                    attached_any = true;
                }
            }
        }
        if attached_any {
            renegotiate(peer).await;
            println!("renegotiate existing tracks for {sub_id}");
        }
    }

    async fn send_track_state(&self, sub_id: Uuid, track: &PublishedTrack) {
        let Some(peer) = self.peers.get(&sub_id) else {
            return;
        };

        if let Err(_) = peer
            .send_control(&TrackState {
                event_type: "track_state",
                from: track.publisher,
                kind: track.kind.to_string(),
                muted: !track.active.load(Ordering::Relaxed),
            })
            .await
        {
            warn!("send track state to {sub_id} failed, ws queue full or closed");
        }
    }

    pub async fn set_track_active(&self, track: &PublishedTrack, active: bool) -> bool {
        let prev = track.active.swap(active, Ordering::Relaxed);
        if prev == active {
            return false;
        }
        warn!(
            "set_track_active {prev} -> {active} publisher={} kind={}",
            track.publisher, track.kind
        );

        self.broadcast_control(
            track.publisher,
            &TrackState {
                event_type: "track_state",
                from: track.publisher,
                kind: track.kind.to_string(),
                muted: !active,
            },
        )
        .await;

        true
    }

    pub async fn publish_track(
        &self,
        publisher: Uuid,
        kind: RtpCodecKind,
        codec: RTCRtpCodec,
        rid: String,
        ssrc: u32,
        track_id: String,
    ) -> Arc<PublishedTrack> {
        let track = {
            let mut entry = self.tracks.entry(publisher).or_default();

            let existing = entry.iter().find(|t| t.kind == kind).cloned();

            if let Some(t) = existing {
                t
            } else {
                let room_peer = self
                    .peers
                    .get(&publisher)
                    .map(|p| p.value().clone())
                    .expect("room peer not found");

                let new_track = Arc::new(PublishedTrack::new(
                    publisher, kind, codec, room_peer, track_id,
                ));
                entry.push(new_track.clone());
                new_track
            }
        };

        let is_new_layer = !track.simulcast_layers.iter().any(|l| l.key() == &rid);
        track.add_layer(rid, ssrc);

        if self.recording.load(Ordering::Relaxed) {
            let _ = crate::sfu::record::start_track_recording(self.id, &track).await;
        }

        if is_new_layer {
            let subs: Vec<(Uuid, Arc<RoomPeer>)> = self
                .peers
                .iter()
                .filter(|p| p.key() != &publisher)
                .map(|p| (*p.key(), p.value().clone()))
                .collect();

            for (id, peer) in subs {
                if self.attach(&track, id, &peer).await {
                    renegotiate(&peer).await;
                    println!("renegotiate");
                }
            }
        }
        track
    }

    async fn attach(&self, t: &Arc<PublishedTrack>, sub_id: Uuid, peer: &RoomPeer) -> bool {
        // Pending joiners live in `peers` so the owner can admit them, but they
        // must not receive tracks before admission: their client has no
        // participant entry yet and would drop the track/state forever.
        if !peer.is_admitted() {
            return false;
        }
        if t.is_output_exist(sub_id) {
            return false;
        }
        let mime = t.codec.load().mime_type.clone();

        // instructs the subscriber's peer driver to add a send track and renegotiate
        // with the client; packets are thereafter fanned out via the returned queue
        let Ok(attached) = peer.peer.attach(t.media_track(), mime).await else {
            warn!("attach track to {sub_id}: peer driver unavailable");
            return false;
        };

        t.add_output(Output {
            sub: sub_id,
            sender_id: attached.sender_id,
            tx: attached.tx,
            svc: Arc::new(LayerSelector::new(0, 2)),
        })
        .await;
        self.send_track_state(sub_id, t).await;
        true
    }

    pub async fn unpublish_track(&self, track: &Arc<PublishedTrack>) {
        track.stop_recording();

        if let Some(mut entry) = self.tracks.get_mut(&track.publisher) {
            entry.retain(|t| !Arc::ptr_eq(t, track));
        }

        let outs = track.renew_outputs();

        // detach from every subscriber's driver; removing the track triggers
        // renegotiation inside the driver itself
        for o in outs.iter() {
            if let Some(peer) = self.peers.get(&o.sub) {
                if let Err(e) = peer.peer.detach(o.sender_id).await {
                    warn!("detach track on {}: {e}", o.sub);
                }
            }
        }

        self.broadcast_control(
            track.publisher,
            &TrackState {
                event_type: "track_ended",
                from: track.publisher,
                kind: track.kind.to_string(),
                muted: false,
            },
        )
        .await;
    }

    pub fn broadcast(&self, conn_id: Uuid, msg: Message) -> Vec<Uuid> {
        let mut failed = Vec::new();

        for peer in self.peers.iter() {
            if peer.key() == &conn_id {
                continue;
            }

            let Some(ws_tx) = peer.ws_tx.clone() else {
                continue;
            };

            if ws_tx.try_send(msg.clone()).is_err() {
                warn!(
                    "broadcast to {} failed, ws queue full or closed",
                    peer.key()
                );
                failed.push(peer.key().to_owned());
            }
        }

        failed
    }

    pub async fn broadcast_control<T>(&self, conn_id: Uuid, msg: &T) -> Vec<Uuid>
    where
        T: sonic_rs::Serialize + ?Sized,
    {
        let mut failed = Vec::new();

        for peer in self.peers.iter() {
            if peer.key() == &conn_id {
                continue;
            }
            if let Err(_) = peer.send_control(msg).await {
                warn!(
                    "broadcast to {} failed, ws queue full or closed",
                    peer.key()
                );
                failed.push(peer.key().to_owned());
            }
        }

        failed
    }

    pub async fn broadcast_chat<T>(&self, conn_id: Uuid, msg: &T) -> Vec<Uuid>
    where
        T: sonic_rs::Serialize + ?Sized,
    {
        let mut failed = Vec::new();

        for peer in self.peers.iter() {
            if peer.key() == &conn_id {
                continue;
            }
            if let Err(_) = peer.send_chat(msg).await {
                warn!(
                    "broadcast to {} failed, ws queue full or closed",
                    peer.key()
                );
                failed.push(peer.key().to_owned());
            }
        }

        failed
    }

    pub fn pending_joiners(&self) -> Vec<(Uuid, Arc<str>)> {
        self.peers
            .iter()
            .filter(|p| !p.value().is_admitted())
            .map(|p| (*p.key(), Arc::clone(&p.value().username)))
            .collect()
    }

    pub fn send_to_owner(&self, from: Uuid, msg: Message) -> Vec<Uuid> {
        let mut failed = Vec::new();

        for peer in self.peers.iter() {
            if peer.key() == &from || peer.user_id != Some(self.owner_id) {
                continue;
            }

            let Some(ws_tx) = peer.ws_tx.clone() else {
                continue;
            };

            if ws_tx.try_send(msg.clone()).is_err() {
                failed.push(peer.key().to_owned());
            }
        }

        failed
    }
}

/// # renegotiate
///
/// renegotiation publisher to client
///
/// kept as a hook: the peer driver itself renegotiates whenever tracks are
/// attached/detached (`OnNegotiationNeededEvent` -> offer -> answer), so the
/// room layer no longer drives offers directly.
pub async fn renegotiate(peer: &RoomPeer) {
    let _ = peer;
}

impl From<&models::room::Room> for RoomSfu {
    fn from(v: &Room) -> Self {
        Self {
            id: v
                .id
                .expect("Room must have an id to be converted into RoomSfu"),
            peers: DashMap::new(),
            alias: v.alias.to_owned(),
            owner_id: v.owner_id,
            tracks: DashMap::new(),
            recording: AtomicBool::new(false),
            pending_joins: AtomicUsize::new(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room() -> RoomSfu {
        RoomSfu {
            id: Uuid::nil(),
            owner_id: Uuid::nil(),
            alias: Arc::from("test"),
            peers: DashMap::new(),
            tracks: DashMap::new(),
            recording: AtomicBool::new(false),
            pending_joins: AtomicUsize::new(0),
        }
    }

    async fn published(publisher: Uuid) -> PublishedTrack {
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let sender_peer = RoomPeer::new(
            None,
            Uuid::new_v4(),
            Arc::from("test"),
            Arc::new(crate::peer::PeerCommandInstance { tx }),
            None,
            Arc::new(room()),
            true,
        )
        .await
        .ok()
        .unwrap();
        PublishedTrack::new(
            publisher,
            RtpCodecKind::Video,
            RTCRtpCodec {
                mime_type: "video/VP8".to_owned(),
                clock_rate: 90000,
                ..Default::default()
            },
            sender_peer,
            "track-test".to_owned(),
        )
    }

    async fn guest() -> Arc<RoomPeer> {
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        RoomPeer::new(
            None,
            Uuid::new_v4(),
            Arc::from("guest"),
            Arc::new(crate::peer::PeerCommandInstance { tx }),
            None,
            Arc::new(room()),
            false,
        )
        .await
        .ok()
        .unwrap()
    }

    /// A pending joiner is in the room map but must not receive tracks until the
    /// owner admits it; otherwise its client drops the early track/state and the
    /// publisher's camera stays "off" forever.
    #[tokio::test]
    async fn pending_peer_does_not_receive_tracks() {
        let room = room();
        let track = Arc::new(published(Uuid::nil()).await);
        let peer = guest().await;
        let sub_id = Uuid::new_v4();

        assert!(!room.attach(&track, sub_id, &peer).await);
        assert!(!track.is_output_exist(sub_id));

        peer.admit();
        assert!(peer.is_admitted());
    }

    /// Pending joiners must be discoverable when the owner reconnects so their
    /// `request` message can be replayed instead of being lost.
    #[tokio::test]
    async fn pending_joiners_lists_only_unadmitted_peers() {
        let room = room();
        let admitted = guest().await;
        admitted.admit();
        let pending = guest().await;

        let admitted_id = Uuid::new_v4();
        let pending_id = Uuid::new_v4();
        assert!(room.add_peer(admitted_id, admitted));
        assert!(room.add_peer(pending_id, pending));

        let joiners = room.pending_joiners();
        assert_eq!(joiners.len(), 1);
        assert_eq!(joiners[0].0, pending_id);
        assert_eq!(&*joiners[0].1, "guest");
    }

    /// A track that just started publishing is active, and every state change must be
    /// reported exactly once. Regression: the track used to start as inactive while the
    /// publisher loop assumed it was active, so turning the camera off was swallowed.
    #[tokio::test]
    async fn track_state_changes_are_reported_once() {
        let room = room();
        let track = published(Uuid::nil()).await;

        assert!(track.is_active());

        // camera off
        assert!(room.set_track_active(&track, false).await);
        assert!(!track.is_active());

        // repeated off is not a state change
        assert!(!room.set_track_active(&track, false).await);
        assert!(!track.is_active());

        // camera on again
        assert!(room.set_track_active(&track, true).await);
        assert!(track.is_active());

        // repeated on is not a state change
        assert!(!room.set_track_active(&track, true).await);
        assert!(track.is_active());
    }
}
