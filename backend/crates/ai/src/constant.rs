// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

use std::time::Duration;

pub(crate) const SAMPLE_RATE: u32 = 16_000;
pub(crate) const SILENCE: Duration = Duration::from_millis(600);
pub(crate) const SPEECH_RMS: f32 = 0.006;
pub(crate) const PAUSE_MS: u32 = 500;
pub(crate) const MAX_SAMPLES: usize = SAMPLE_RATE as usize * 10;
pub(crate) const MIN_SAMPLES: usize = SAMPLE_RATE as usize / 2;
