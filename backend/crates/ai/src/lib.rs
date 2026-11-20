// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

use std::env::VarError;

mod constant;
pub mod core;
pub mod lang;
pub mod onnx;
pub mod stt;
pub mod tts;

pub struct AiError(pub String);

impl From<VarError> for AiError {
    fn from(value: VarError) -> Self {
        AiError(value.to_string())
    }
}

impl From<AiError> for Box<dyn std::error::Error + Send + Sync> {
    fn from(value: AiError) -> Self {
        Box::new(std::io::Error::other(value.0))
    }
}
