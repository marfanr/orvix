// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

use std::time::Duration;

pub const CONTROL_LABEL: &str = "control";
pub const MESSAGE_LABEL: &str = "chat";

// TODO: chnag einto dynamically per peer
/// the simulcast layer that gets forwarded to subscribers; layers whose rid
/// differs are demoted here (audio tracks without rid are normalized to this)
pub const FORWARD_RID: &str = "q";

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(86400);
/// per-subscriber RTP fan-out queue; dropping on overflow instead of backpressuring
pub const MEDIA_QUEUE_SIZE: usize = 256;
/// max outbound packets written per loop iteration, so media cannot starve signaling
pub const MEDIA_BATCH: usize = 64;
pub const PEER_QUEUE: usize = 1024;
/// max commands applied per loop iteration
pub const CMD_BATCH: usize = 32;
/// how long to wait for the answer to a renegotiation offer before issuing a
/// fresh one; without this an unanswered offer pins `offer_outstanding` forever
pub const OFFER_TIMEOUT: Duration = Duration::from_secs(10);
/// minimum spacing between server-initiated ICE restarts for one leg
pub const ICE_RESTART_COOLDOWN: Duration = Duration::from_secs(5);
/// give up (close the driver) after this many back-to-back ICE restarts
pub const MAX_ICE_RESTARTS: u32 = 5;
/// how long before an inbound track is considered dead
pub const IDLE_TIMEOUT: Duration = Duration::from_millis(800);
/// minimum interval between PLI requests on a given track
pub const PLI_INTERVAL: Duration = Duration::from_millis(650);
/// window during which a resumed track keeps asking for keyframes
pub const PLI_WINDOW: Duration = Duration::from_secs(3);
