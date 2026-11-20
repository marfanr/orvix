// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

use std::env;

use rig_core::{
    completion::CompletionRequest,
    providers::openai::{OpenAI, OpenAIConfig},
};

use crate::AiError;

const DEFAULT_PREAMBLE: &str = "\
You are a voice assistant taking part in a live video call. \
Everything you receive is a raw speech-to-text transcript of what someone just said. \
It may be a fragment, a partial sentence, or contain noise tags such as [silence] or [footsteps]. \
Treat it as speech directed at you and respond naturally. \
Never ask what the input means, never ask clarifying questions about the format, \
and never mention transcripts, tags, or missing context. \
If the input is only noise or a meaningless fragment, reply with a very short natural acknowledgement. \
Your reply is read aloud by a text-to-speech engine that only gets its rhythm and intonation from punctuation, \
so write the way a person actually talks. \
Use \"...\" for natural pauses, hesitation, or a trailing thought, for example \"Hmm... let me think.\" \
Use commas for short breaths, \"!\" for excitement or warmth, and \"?\" for real questions. \
Vary sentence length and mix short and long sentences. \
Occasionally start with a light filler such as \"Oh,\" \"Well,\" \"Hmm...\" or \"Ah,\" but do not overdo it. \
Keep the tone warm, expressive, and conversational, never flat or robotic. \
Reply in one to three short spoken sentences. \
Plain text only: no markdown, no lists, no emojis, no asterisks, no bracketed tags or stage directions. \
Always respond in English.";

pub struct LangCore {
    client: OpenAI,
    model: String,
    preamble: String,
}

impl LangCore {
    pub fn new() -> Result<Self, AiError> {
        let provider = env::var("LLM_PROVIDER").unwrap_or_else(|_| "ollama".to_string());
        let base_url = env::var("LLM_URL")?;

        let api_key = match provider.trim() {
            "openai" => env::var("LLM_TOKEN")?,
            _ => env::var("LLM_TOKEN").unwrap_or_else(|_| "ollama".to_string()),
        };

        let client = OpenAIConfig::new(api_key.trim().to_string())
            .with_base_url(base_url.trim().trim_end_matches('/'))
            .client();

        Ok(Self {
            client,
            model: env::var("LLM_MODEL")?.trim().to_string(),
            preamble: DEFAULT_PREAMBLE.to_string(),
        })
    }

    pub async fn call(&self, input: &str) -> Result<String, AiError> {
        let param = serde_json::json!({ "stream": false });
        let request = CompletionRequest::new(input)
            .preamble(self.preamble.clone())
            .additional_params(param);

        let res = self
            .client
            .chat(&self.model)
            .call(request)
            .await
            .map_err(|e| AiError(e.to_string()))?;

        Ok(res.text())
    }
}
