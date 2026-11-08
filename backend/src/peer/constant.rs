use std::time::Duration;

pub const CONTROL_LABEL: &str = "control";
pub const MESSAGE_LABEL: &str = "message";

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(86400);
/// per-subscriber RTP fan-out queue; dropping on overflow instead of backpressuring
pub const MEDIA_QUEUE_SIZE: usize = 256;
/// max outbound packets written per loop iteration, so media cannot starve signaling
pub const MEDIA_BATCH: usize = 64;
pub const PEER_QUEUE: usize = 1024;
/// max commands applied per loop iteration
pub const CMD_BATCH: usize = 32;
/// how long before an inbound track is considered dead
pub const IDLE_TIMEOUT: Duration = Duration::from_millis(500);
/// minimum interval between PLI requests on a given track
pub const PLI_INTERVAL: Duration = Duration::from_millis(650);
/// window during which a resumed track keeps asking for keyframes
pub const PLI_WINDOW: Duration = Duration::from_secs(3);
