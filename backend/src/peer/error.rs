use std::fmt;

pub enum PeerError {
    DriverError(String),
}

impl fmt::Display for PeerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PeerError::DriverError(msg) => write!(f, "{msg}"),
        }
    }
}

impl From<PeerError> for String {
    fn from(err: PeerError) -> String {
        err.to_string()
    }
}