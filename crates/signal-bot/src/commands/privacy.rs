//! Privacy / TEE explanation menu.

use crate::commands::menu_locale::privacy_menu;
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
    fn trigger(&self) -> Option<&str> {
        Some("!privacy")
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
        let handler = PrivacyHandler::new(BotRole::Translation);
        let out = handler.execute(&dm("!privacy")).await.unwrap();
        assert!(out.contains("Bread Bot translation"));
        assert!(out.contains("!verify"));
    }
}
