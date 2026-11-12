// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

use crate::api::WsError;
use crate::api::server::ServerState;
use crate::models;
use crate::sfu::room::RoomSfu;
use crate::sfu::room_peer::RoomPeer;
use crate::utils::{AccessClaims, verify_jwt};
use axum::Router;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::get;
use futures_util::{TryFutureExt, sink::SinkExt, stream::StreamExt};
use http::header::{ORIGIN, USER_AGENT};
use http::{HeaderMap, StatusCode};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsMsg {
    Join {
        room: Uuid,
        token: Option<Arc<str>>,
        /// username only used if token is empty
        username: Option<Arc<str>>,
    },
    AnswerRequest {
        conn_id: Uuid,
        room_id: Uuid,
    },
    Offer {
        sdp: String,
    },
    Answer {
        sdp: String,
    },
    Candidate {
        from: String,
        candidate: String,
        mid: Option<String>,
        line_index: Option<u16>,
        username_fragment: Option<String>,
    },
    Leave,
}

async fn answer_join(
    sink: &mpsc::Sender<Message>,
    accept: bool,
    conn_id: Uuid,
) -> Result<(), WsError> {
    sink.send(Message::text(
        sonic_rs::json!({ "type": "join", "accept": accept, "conn_id": conn_id}).to_string(),
    ))
    .map_err(|e| WsError::from(e))
    .await
}

async fn sync_room_member(
    sink: &mpsc::Sender<Message>,
    r: &RoomSfu,
    conn_id: &Uuid,
) -> Result<(), WsError> {
    let members: Vec<_> = r
        .peers
        .iter()
        .filter(|p| p.key() != conn_id && p.value().is_admitted())
        .map(|p| {
            sonic_rs::json!({
                "conn_id": p.key(),
                "username": &*p.value().username,
            })
        })
        .collect();

    sink.send(Message::text(
        sonic_rs::json!({ "type": "sync", "members": members}).to_string(),
    ))
    .map_err(|e| WsError::from(e))
    .await
}

impl WsMsg {
    /// handle joining a room
    async fn handle_join(
        room_id: Uuid,
        conn_id: Uuid,
        token: Option<Arc<str>>,
        username: Option<Arc<str>>,
        state: &ServerState,
        self_sink: mpsc::Sender<Message>,
    ) -> Result<(), WsError> {
        let user_id = token
            .as_deref()
            .map(verify_jwt::<AccessClaims>)
            .transpose()
            .map_err(|e| WsError(Arc::from(e.to_string())))?
            .map(|c| c.sub.parse::<Uuid>())
            .transpose()
            .map_err(|e| WsError(Arc::from(e.to_string())))?;

        info!("join room={} from {} ", room_id, conn_id);

        // created room object on memory only if owners already joined
        let db = state.pool.get().map_err(|e| WsError::from(e)).await?;

        // gets the SFU Room exist, or create it from db if available.
        // `register_join` is called while holding the `state.rooms` shard guard so
        // a concurrent teardown of the last peer cannot remove the room between
        // us reading it and marking it as in-use.
        let existing = state.rooms.get(&room_id).map(|r| {
            let room = Arc::clone(r.value());
            let token = room.register_join();
            (room, token)
        });
        let (room_arc, _join_token) = match existing {
            Some(pair) => pair,
            None => {
                let model = models::room::Room::get(&db, room_id).await?;
                let fresh = Arc::new(RoomSfu::from(&model));

                if user_id != Some(fresh.owner_id) {
                    return Err(WsError(Arc::from("room not started")));
                }

                {
                    let entry = state.rooms.entry(room_id).or_insert(fresh);
                    let room = Arc::clone(entry.value());
                    let token = room.register_join();
                    (room, token)
                }
            }
        };
        let room: &RoomSfu = room_arc.as_ref();

        let username = username.unwrap_or(Arc::from("Anonymous"));
        let username = match user_id {
            Some(id) => models::user::User::get_by_id(&db, id)
                .await
                .map(|u| u.username)
                .unwrap_or_else(|e| {
                    tracing::warn!(%id, error = %e, "failed to load user, using fallback name");
                    username
                }),
            None => username,
        };

        //                 WebSocket (signaling: offer / answer / ice + `pc` field)
        //        +---------------------------------------------+
        //        |                                             |
        //   +---------+                                   +---------+
        //   | Browser |                                   |   SFU   |
        //   +---------+                                   +---------+
        //        |                                             |
        //        |  pubPc  (Publisher)                         |
        //        |  offer  ------------------------------->    |  browser offers
        //        |  <-------------------------------  answer   |  SFU only answers
        //        |  camera/mic RTP (simulcast q/h) ------->    |  -> on_track
        //        |                                             |
        //        |  subPc  (Subscriber)                        |
        //        |  <-------------------------------  offer    |  SFU offers
        //        |  answer  ------------------------------->   |  browser only answers
        //        |  <------ RTP of other peers (Output)        |  <- attach()
        //        |  <------ control channel (track_state)      |

        // TODO: migration to RTC sans-io
        let new_peer = crate::peer::create_peer(
            conn_id,
            state.announce_ip,
            Arc::clone(&room_arc),
            self_sink.clone(),
        );

        let admitted = user_id == Some(room.owner_id);
        let room_peer = RoomPeer::new(
            user_id,
            conn_id,
            Arc::clone(&username),
            new_peer,
            Some(self_sink.clone()),
            Arc::clone(&room_arc),
            admitted,
        )
        .await
        .map_err(|e| WsError(Arc::from(format!("room peer: {}", e.to_string()))))?;

        room.add_peer(conn_id, room_peer);
        state.conn_to_rooms.insert(conn_id, Arc::clone(&room_arc));

        // handle joining room request
        // if owner directy enter the room
        // TODO: give permission for another user, instead for room owner only
        if user_id == Some(room.owner_id) {
            answer_join(&self_sink, true, conn_id).await?;
            tracing::info!("Owner joined , current member room : {}", room.peers.len());

            // reply request
            for (pending_conn, pending_name) in room.pending_joiners() {
                let request = sonic_rs::json!({
                    "type": "request",
                    "from": {"conn_id": pending_conn, "username": pending_name},
                })
                .to_string();

                if let Err(e) = self_sink.send(Message::text(request)).await {
                    warn!("replay join request for {pending_conn}: {e}");
                }
            }

            // resend sync into self and each member
            _ = sync_room_member(&self_sink, &room, &conn_id)
                .await
                .map_err(|e| warn!("sync room member: {e}"));

            for m in room.peers.iter() {
                let Some(ws) = m.ws_tx.clone() else { continue };
                _ = sync_room_member(&ws, &room, &m.key())
                    .await
                    .map_err(|e| warn!("sync room member: {e}"));
            }

            state
                .broadcast_to_waiting_room(
                    room_id,
                    Message::text(sonic_rs::json!({"type":"notify"}).to_string()),
                )
                .await;

            return Ok(());
        }

        // drop self from waiting room
        state.drop_from_waiting_room(room_id, self_sink);

        // if not owner send join requets to the owner of room
        // TODO!: check on options first, is the room need request to join or can join directly
        room.send_to_owner(
            conn_id,
            Message::text(
                sonic_rs::json!({
                "type": "request", "from": {"conn_id": conn_id, "username": username}})
                .to_string(),
            ),
        );

        Ok(())
    }

    /// Offering into subscriber pc
    async fn handle_offer(
        sdp: &str,
        conn_id: Uuid,
        state: &ServerState,
        self_sink: mpsc::Sender<Message>,
    ) -> Result<(), String> {
        let room = state
            .conn_to_rooms
            .get(&conn_id)
            .ok_or_else(|| "room not found for conn".to_string())?;
        let peers = room
            .peers
            .get(&conn_id)
            .ok_or_else(|| "peer not found in room".to_string())?;
        let peer = Arc::clone(&peers.peer);

        let answer = peer
            .answer_offer(sdp.to_string(), crate::peer::PeerType::SUBSCRIBER)
            .await?;

        self_sink
            .send(Message::text(
                sonic_rs::json!({ "type": "answer", "sdp": answer }).to_string(),
            ))
            .await
            .map_err(|e| format!("send answer: {e}"))?;

        info!("new offer");

        // After initial offer/answer negotiation, attach any existing tracks from other peers
        room.attach_existing_tracks(conn_id, &peers).await;

        Ok(())
    }

    async fn handle_answer(sdp: &str, conn_id: Uuid, state: &ServerState) -> Result<(), String> {
        let room = state
            .conn_to_rooms
            .get(&conn_id)
            .ok_or_else(|| "room not found for conn".to_string())?;
        let peers = room
            .peers
            .get(&conn_id)
            .ok_or_else(|| "peer not found in room".to_string())?;
        let pc = Arc::clone(&peers.peer);

        pc.apply_answer(sdp.to_string()).await?;
        Ok(())
    }
}

pub fn routes() -> Router<ServerState> {
    Router::new().route("/", get(self::ws))
}

async fn ws(
    State(s): State<ServerState>,
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    State(state): State<ServerState>,
) -> Result<Response, StatusCode> {
    let origin = headers
        .get(ORIGIN)
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::FORBIDDEN)?;

    if !s.allowed_origins.iter().any(|o| o == origin) {
        return Err(StatusCode::FORBIDDEN);
    }

    let user_agent = headers
        .get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("Unknown")
        .to_owned();

    let conn_id = Uuid::now_v7();
    Ok(ws.on_upgrade(move |socket| self::handle_socket(socket, user_agent, state, conn_id)))
}

async fn handle_socket(socket: WebSocket, _user_agent: String, state: ServerState, conn_id: Uuid) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::channel::<Message>(64);
    state.ws_conn.insert(conn_id, tx.clone());

    loop {
        tokio::select! {
                incoming = stream.next() => {
                    match incoming {
                        Some(Ok(Message::Close(_)) | Err(_)) | None => break,
                        Some(Ok(Message::Text(msg))) => {
                            match sonic_rs::from_slice::<WsMsg>(msg.as_bytes()) {
                                Ok(WsMsg::Join { room, token, username }) => {
                                    if let Err(e) = WsMsg::handle_join(
                                        room,
                                        conn_id,token, username, &state, tx.clone()).await {
                                        warn!("handle_join {}: {e}", room);

                                        state.waiting_room.entry(room).or_default().push(tx.clone());
                                    }
                                }
                                // only forwarding the message from client
                                Ok(WsMsg::AnswerRequest {conn_id, room_id}) => {
                                    // Clone the handles out of the DashMap guards before
                                    // awaiting: holding a guard across `.await` can block
                                    // every other task that touches the same shard.
                                    let joiner_tx = state
                                        .ws_conn
                                        .get(&conn_id)
                                        .map(|c| c.value().clone());
                                    let room = state
                                        .rooms
                                        .get(&room_id)
                                        .map(|r| Arc::clone(r.value()));

                                    if let (Some(joiner_tx), Some(room)) = (joiner_tx, room) {
                                        // Only admit a connection that is still a member.
                                        // The joiner may have disconnected while the host was
                                        // deciding; announcing it afterwards would broadcast a
                                        // `welcome` *after* its `disconnect` -> ghost member.
                                        let peer = room.peers.get(&conn_id).map(|p| {
                                            (Arc::clone(&p.username), Arc::clone(p.value()))
                                        });
                                        if let Some((username, peer)) = peer {
                                            if let Err(e) =
                                                answer_join(&joiner_tx, true, conn_id).await
                                            {
                                                warn!("answer_join: {e}");
                                            } else if let Err(e) =
                                                sync_room_member(&joiner_tx, &room, &conn_id).await
                                            {
                                                warn!("sync room member: {e}");
                                            } else {
                                                // Admit only after `join` and `sync` are
                                                // queued: from now on tracks may attach to
                                                // this peer, and its client must already know
                                                // the room members.
                                                peer.admit();
                                                // The joiner may have joined with no local
                                                // media, so it will not necessarily send an
                                                // offer that would trigger attachment.
                                                room.attach_existing_tracks(conn_id, &peer).await;
                                                if room.peers.contains_key(&conn_id) {
                                                    room.broadcast(conn_id, Message::text(
                                                        sonic_rs::json!({
                                                        "type": "welcome", "from": {"conn_id": conn_id, "username": username}})
                                                        .to_string(),
                                                    ));
                                                }
                                            }
                                        } else {
                                            warn!("admit {conn_id}: peer already left");
                                        }
                                    };
                                }
                                Ok(WsMsg::Offer { sdp }) => {
                                    if let Err(e) = WsMsg::handle_offer(&sdp, conn_id, &state, tx.clone()).await {
                                        warn!("handle_offer: {e}");
                                    }
                                }
                                Ok(WsMsg::Answer { sdp }) => {
                                    if let Err(e) = WsMsg::handle_answer(&sdp, conn_id, &state).await {
                                        warn!("handle_answer: {e}");
                                    }
                                }
                                Ok(WsMsg::Candidate { from, candidate, mid, line_index, username_fragment }) => {
                                    // clone the peer connection out of the map before awaiting,
                                    // holding a dashmap guard across an await point can deadlock
                                    let room_peer = state
                                        .conn_to_rooms
                                        .get(&conn_id)
                                        .and_then(|r| r.peers.get(&conn_id).map(|p| Arc::clone(p.value())));


                                    if let Some(p) = room_peer {
                                        // client('PUBLISHER') media masuk ke leg SUBSCRIBER,
                                        // client('SUBSCRIBER') menerima dari leg PUBLISHER
                                        let pc_type = if from == "SUBSCRIBER" {
                                            crate::peer::PeerType::PUBLISHER
                                        } else {
                                            crate::peer::PeerType::SUBSCRIBER
                                        };
                                        _ = p.peer.add_remote_candidate(
                                            candidate,
                                            mid,
                                            line_index,
                                            username_fragment,
                                            pc_type,
                                        ).await;
                                    }
                                }
                                Ok(WsMsg::Leave) => break,
                                Err(e) => warn!("bad ws msg: {e}"),
                            }
                        }
                        Some(Ok(_)) => {}
                    }
                }

            outgoing = rx.recv() => {
                match outgoing {
                    Some(msg) => {
                        if sink.send(msg).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    // clear connection
    state.ws_conn.remove(&conn_id);

    // Take the room out of the conn map first: this releases the DashMap guard
    // before any `.await` (holding it across awaits can block other tasks) and
    // makes stale `conn_id -> room` lookups stop resolving immediately.
    let room = state.conn_to_rooms.remove(&conn_id).map(|(_, room)| room);

    if let Some(room) = room {
        println!("delete {} from room", &conn_id);
        let username = room
            .peers
            .get(&conn_id)
            .map(|p| p.username.clone())
            .unwrap_or(Arc::from("Anonymous"));

        // Remove the peer BEFORE broadcasting the disconnect. `sync_room_member`
        // snapshots `room.peers`, so if the disconnect is broadcast first, a peer
        // joining in between would receive a member list that still contains this
        // conn and would never see the disconnect event -> ghost member.
        room.drop_peer(conn_id).await;

        room.broadcast(
            conn_id,
            Message::text(
                sonic_rs::json !({
                "type": "disconnect", "conn_id": conn_id, "username": username
                })
                .to_string(),
            ),
        );

        // Delete the room only if it is still empty. `remove_if` is atomic per
        // shard and `can_be_removed` also accounts for joins that are in flight,
        // so a joiner that grabbed this room cannot end up in an orphaned room.
        // Note: the key is `room.id`, not the `conn_to_rooms` key.
        state
            .rooms
            .remove_if(&room.id, |_, r| Arc::ptr_eq(r, &room) && r.can_be_removed());
    }

    info!("ws disconnected");
}
