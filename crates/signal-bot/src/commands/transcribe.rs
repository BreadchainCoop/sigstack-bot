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
        self.store.set_enabled(context_id, enable, message.is_group);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::group_preferences_store::GroupPreferencesStore;

    fn msg(text: &str, group: bool) -> BotMessage {
        BotMessage {
            source: "+15550002222".into(),
            source_number: Some("+15550002222".into()),
            source_name: None,
            text: text.into(),
            timestamp: 1,
            message_timestamp: 1,
            is_group: group,
            group_id: group.then(|| "group-1".into()),
            group_name: None,
            receiving_account: "+15550001111".into(),
            attachments: vec![],
            quote: None,
        }
    }

    #[test]
    fn matches_on_off_only() {
        let store = Arc::new(TranscribeStore::new(None));
        let handler = TranscribeHandler::new(store, true);
        assert!(handler.matches(&msg("!transcribe-on", false)));
        assert!(handler.matches(&msg("!transcribe-off", true)));
        assert!(!handler.matches(&msg("!transcribe", false)));
        assert!(!handler.matches(&msg("!help", false)));
    }

    #[tokio::test]
    async fn execute_toggles_dm_and_group() {
        let prefs = GroupPreferencesStore::new_in_memory(0);
        let store = Arc::new(TranscribeStore::new(Some(prefs)));
        let handler = TranscribeHandler::new(store.clone(), true);

        assert_eq!(
            handler
                .execute(&msg("!transcribe-off", false))
                .await
                .unwrap(),
            "Voice transcription disabled."
        );
        assert!(!store.is_enabled("+15550002222", false));

        assert_eq!(
            handler.execute(&msg("!transcribe-on", true)).await.unwrap(),
            "Voice transcription enabled for this group."
        );
        assert!(store.is_enabled("group-1", true));

        assert_eq!(
            handler
                .execute(&msg("!transcribe-off", true))
                .await
                .unwrap(),
            "Voice transcription disabled for this group."
        );
    }

    #[tokio::test]
    async fn execute_reports_unavailable_when_whisper_off() {
        let store = Arc::new(TranscribeStore::new(None));
        let handler = TranscribeHandler::new(store, false);
        let out = handler
            .execute(&msg("!transcribe-on", false))
            .await
            .unwrap();
        assert!(out.contains("not available"));
    }
}
