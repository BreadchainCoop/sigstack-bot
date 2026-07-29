//! `!privacy` — privacy, security, and TEE commands menu.

use crate::commands::menu_locale::{menu_language_for_message, privacy_menu};
use crate::commands::CommandHandler;
use crate::config::BotRole;
use crate::error::AppResult;
use crate::group_preferences_store::GroupPreferencesStore;
use async_trait::async_trait;
use signal_client::BotMessage;
use std::sync::Arc;

pub struct PrivacyHandler {
    group_prefs: Arc<GroupPreferencesStore>,
    role: BotRole,
}

impl PrivacyHandler {
    pub fn new(group_prefs: Arc<GroupPreferencesStore>, role: BotRole) -> Self {
        Self { group_prefs, role }
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

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        let language = menu_language_for_message(message, &self.group_prefs);
        Ok(privacy_menu(language, self.role).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dm(text: &str) -> BotMessage {
        BotMessage {
            source: "+15550002222".into(),
            source_number: None,
            source_name: None,
            text: text.into(),
            timestamp: 1,
            message_timestamp: 1,
            is_group: false,
            group_id: None,
            group_name: None,
            receiving_account: "+15550001111".into(),
            attachments: vec![],
            quote: None,
        }
    }

    #[tokio::test]
    async fn privacy_returns_role_menus() {
        let store = GroupPreferencesStore::new_in_memory(0);
        let transcription = PrivacyHandler::new(store.clone(), BotRole::Transcription);
        let translation = PrivacyHandler::new(store, BotRole::Translation);

        assert!(transcription.matches(&dm("!privacy")));
        let t = transcription.execute(&dm("!privacy")).await.unwrap();
        assert!(t.contains("Sigstack transcription"));
        assert!(t.contains("!verify"));

        let t = translation.execute(&dm("!privacy")).await.unwrap();
        assert!(t.contains("Sigstack translation"));
        assert!(t.contains("!verify"));
        assert!(!t.contains("!models"));
    }
}
