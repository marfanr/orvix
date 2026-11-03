// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

pub mod errors;
pub mod peer;
pub mod record;
pub mod room;
pub mod track;
mod data_channel;
pub mod room_peer;

#[derive(Debug, Copy, Clone)]
pub enum PeerType {
    PUBLISHER,
    SUBSCRIBER,
}

impl PeerType {
    pub fn get_opposite(&self) -> Self {
        match self {
            PeerType::SUBSCRIBER => PeerType::PUBLISHER,
            _ => PeerType::PUBLISHER,
        }
    }

    pub fn to_string(&self) -> String {
        String::from(self)
    }

    pub fn is_subscriber(&self) -> bool {
        matches!(self, PeerType::SUBSCRIBER)
    }

    pub fn is_publisher(&self) -> bool {
        matches!(self, PeerType::PUBLISHER)
    }
}

impl From<&PeerType> for String {
    fn from(value: &PeerType) -> Self {
        match value {
            PeerType::PUBLISHER => String::from("PUBLISHER"),
            PeerType::SUBSCRIBER => String::from("SUBSCRIBER"),
        }
    }
}