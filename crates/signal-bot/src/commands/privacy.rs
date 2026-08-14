//! Privacy / TEE explanation menu (translation hub only).

use crate::commands::menu_locale::{is_exact_command, privacy_menu};
use crate::commands::CommandHandler;
use crate::error::AppResult;
use async_trait::async_trait;
use signal_client::BotMessage;

pub struct PrivacyHandler;

impl PrivacyHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PrivacyHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandHandler for PrivacyHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        is_exact_command(&message.text, "!privacy")
    }

    fn label(&self) -> &'static str {
        "privacy"
    }

    async fn execute(&self, _message: &BotMessage) -> AppResult<String> {
        Ok(privacy_menu().into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dm(text: &str) -> BotMessage {
        BotMessage {
            source: "+1".into(),
            source_number: None,
            source_name: None,
            text: text.into(),
            timestamp: 0,
            message_timestamp: 0,
            is_group: false,
            group_id: None,
            group_name: None,
            receiving_account: "+2".into(),
            attachments: vec![],
            quote: None,
        }
    }

    #[tokio::test]
    async fn privacy_returns_unified_menu() {
        let handler = PrivacyHandler::new();
        assert!(handler.matches(&dm("!privacy")));
        assert!(!handler.matches(&dm("!privacy-translation")));
        assert!(!handler.matches(&dm("!privacy-transcription")));
        let out = handler.execute(&dm("!privacy")).await.unwrap();
        assert!(out.contains("one Phala TEE/CVM"));
        assert!(out.contains("NEAR AI Whisper"));
        assert!(out.contains("!verify"));
        assert!(!out.contains("**"));
    }
}
