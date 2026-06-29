//! `!transcribe-on` / `!transcribe-off` — per-chat voice transcription toggle.

use crate::commands::CommandHandler;
use crate::error::AppResult;
use crate::transcribe_store::TranscribeStore;
use async_trait::async_trait;
use signal_client::BotMessage;
use std::sync::Arc;

pub struct TranscribeHandler {
    store: Arc<TranscribeStore>,
    whisper_available: bool,
}

impl TranscribeHandler {
    pub fn new(store: Arc<TranscribeStore>, whisper_available: bool) -> Self {
        Self {
            store,
            whisper_available,
        }
    }
}

#[async_trait]
impl CommandHandler for TranscribeHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        let text = message.text.trim();
        text == "!transcribe-on" || text == "!transcribe-off"
    }

    fn label(&self) -> &'static str {
        "transcribe"
    }

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        if !self.whisper_available {
            return Ok("Voice transcription is not available on this bot.".into());
        }

        let context_id = message.reply_target();
        let enable = message.text.trim() == "!transcribe-on";
        self.store
            .set_enabled(context_id, enable, message.is_group);

        if message.is_group {
            if enable {
                Ok("Voice transcription enabled for this group.".into())
            } else {
                Ok("Voice transcription disabled for this group.".into())
            }
        } else if enable {
            Ok("Voice transcription enabled.".into())
        } else {
            Ok("Voice transcription disabled.".into())
        }
    }
}
