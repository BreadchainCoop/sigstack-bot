//! Help command - displays feature menu.

use crate::commands::menu_locale::{help_menu, menu_language_for_message};
use crate::commands::CommandHandler;
use crate::config::BotRole;
use crate::error::AppResult;
use crate::group_preferences_store::GroupPreferencesStore;
use async_trait::async_trait;
use signal_client::BotMessage;
use std::sync::Arc;

pub struct HelpHandler {
    group_prefs: Arc<GroupPreferencesStore>,
    role: BotRole,
}

impl HelpHandler {
    pub fn new(group_prefs: Arc<GroupPreferencesStore>, role: BotRole) -> Self {
        Self { group_prefs, role }
    }
}

#[async_trait]
impl CommandHandler for HelpHandler {
    fn trigger(&self) -> Option<&str> {
        Some("!help")
    }

    fn label(&self) -> &'static str {
        "help"
    }

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        let language = menu_language_for_message(message, &self.group_prefs);
        Ok(help_menu(language, self.role).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu_language::MenuLanguage;

    fn dm(text: &str) -> BotMessage {
        BotMessage {
            source: "+15550002222".into(),
            source_number: Some("+15550002222".into()),
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

    fn group(text: &str, group_id: &str) -> BotMessage {
        let mut m = dm(text);
        m.is_group = true;
        m.group_id = Some(group_id.into());
        m
    }

    #[tokio::test]
    async fn help_returns_role_specific_menu() {
        let store = GroupPreferencesStore::new_in_memory(0);
        let transcription = HelpHandler::new(store.clone(), BotRole::Transcription);
        let translation = HelpHandler::new(store.clone(), BotRole::Translation);

        assert!(transcription.matches(&dm("!help")));
        let t = transcription.execute(&dm("!help")).await.unwrap();
        assert!(t.contains("!transcribe"));
        assert!(!t.contains("!translate-me-on"));

        let t = translation.execute(&dm("!help")).await.unwrap();
        assert!(t.contains("!translate-me-on"));
        assert!(!t.contains("Voice notes in this chat"));
    }

    #[tokio::test]
    async fn help_uses_group_menu_language() {
        let store = GroupPreferencesStore::new_in_memory(0);
        store.set_menu_language("g1", MenuLanguage::Es);
        let handler = HelpHandler::new(store, BotRole::Transcription);
        let out = handler.execute(&group("!help", "g1")).await.unwrap();
        assert!(out.contains("Transcripción de voz"));
    }
}
