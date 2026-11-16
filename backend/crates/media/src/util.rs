// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

use std::io::Cursor;

use rtc::sdp::SessionDescription;
use rtc::stun::attributes::ATTR_USERNAME;
use rtc::stun::message::{Message, is_stun_message};
use rtc::stun::textattrs::TextAttribute;
use rtp::codecs::h264;

pub const AUDIO_LEVEL_URI: &str = "urn:ietf:params:rtp-hdrext:ssrc-audio-level";

pub fn describe_nal(payload: &[u8]) -> Option<String> {
    let b0 = *payload.first()?;
    let h = oxideav_bitstream::h264::nal_header(b0);

    match h.2 {
        t @ 1..=23 => Some(format!("single type={t}")),
        h264::STAPA_NALU_TYPE => Some("STAP-A".to_string()),
        h264::FUA_NALU_TYPE => {
            let fu = *payload.get(1)?;
            let start = fu & 0x80 != 0;
            let end = fu & 0x40 != 0;
            let t = fu & 0x1f;
            Some(format!("FU-A type={t} start={start} end={end}"))
        }
        t => Some(format!("other type={t}")),
    }
}

pub fn stun_local_ufrag(data: &[u8]) -> Option<String> {
    if !is_stun_message(data) {
        return None;
    }
    let mut msg = Message::new();
    msg.unmarshal_binary(data).ok()?;
    let username = TextAttribute::get_from_as(&msg, ATTR_USERNAME).ok()?;
    username.text.split(':').next().map(str::to_owned)
}

pub fn ice_ufrag(sdp: &str) -> Option<&str> {
    sdp.lines()
        .find_map(|l| l.trim().strip_prefix("a=ice-ufrag:"))
}

pub fn find_audio_level_ext_id(sdp: &str) -> Option<u8> {
    let desc = SessionDescription::unmarshal(&mut Cursor::new(sdp.as_bytes())).ok()?;

    desc.media_descriptions
        .iter()
        .filter(|m| m.media_name.media == "audio")
        .flat_map(|m| m.attributes.iter())
        .filter(|a| a.key == "extmap")
        .filter_map(|a| a.value.as_deref())
        .find_map(|v| {
            let mut it = v.split_whitespace();
            let id = it.next()?.split('/').next()?.parse::<u8>().ok()?;
            (it.next()? == AUDIO_LEVEL_URI).then_some(id)
        })
}
