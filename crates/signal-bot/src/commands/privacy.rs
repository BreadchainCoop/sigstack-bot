//! Privacy / TEE explanation menu.

use crate::commands::menu_locale::{is_exact_command, privacy_command, privacy_menu};
use crate::commands::CommandHandler;
use crate::config::BotRole;
use crate::error::AppResult;
use async_trait::async_trait;
use signal_client::BotMessage;

pub struct PrivacyHandler {
    role: BotRole,
}

impl PrivacyHandler {
    pub fn new(role: BotRole) -> Self {
        Self { role }
    }
}

#[async_trait]
impl CommandHandler for PrivacyHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        is_exact_command(&message.text, privacy_command(self.role))
    }

    fn label(&self) -> &'static str {
        "privacy"
    }

    async fn execute(&self, _message: &BotMessage) -> AppResult<String> {
        Ok(privacy_menu(self.role).into())
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
    async fn privacy_returns_role_menu() {
        let translation = PrivacyHandler::new(BotRole::Translation);
        assert!(translation.matches(&dm("!privacy-translation")));
        assert!(!translation.matches(&dm("!privacy")));
        assert!(!translation.matches(&dm("!privacy-transcription")));
        let out = translation
            .execute(&dm("!privacy-translation"))
            .await
            .unwrap();
        assert!(out.contains("Bread Bot translation"));
        assert!(out.contains("!verify"));

        let transcription = PrivacyHandler::new(BotRole::Transcription);
        assert!(transcription.matches(&dm("!privacy-transcription")));
        assert!(!transcription.matches(&dm("!privacy-translation")));
        let out = transcription
            .execute(&dm("!privacy-transcription"))
            .await
            .unwrap();
        assert!(out.contains("Bread Bot transcription"));
    }
}
