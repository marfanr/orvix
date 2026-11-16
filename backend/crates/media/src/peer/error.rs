// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

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
