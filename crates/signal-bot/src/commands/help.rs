//! Help command - displays feature menu.

use crate::commands::menu_locale::{help_menu, thread_help_menu};
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
        if self.role == BotRole::Translation {
            if let Some(group_id) = message.group_id.as_deref() {
                if self.group_prefs.lookup_sidecar(group_id).is_some() {
                    return Ok(thread_help_menu().into());
                }
            }
        }
        Ok(help_menu(self.role).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(t.contains("!translation"));
        assert!(t.contains("!transcription"));
        assert!(t.contains("!privacy"));
        assert!(!t.contains("Voice notes in this chat"));
        assert!(!t.contains("!set-en"));
    }

    #[tokio::test]
    async fn help_in_sidecar_returns_thread_menu() {
        let store = GroupPreferencesStore::new_in_memory(0);
        store.set_sidecar("main-1", "it", "group.it".into(), "it-internal".into());
        let handler = HelpHandler::new(store, BotRole::Translation);
        let out = handler
            .execute(&group("!help", "it-internal"))
            .await
            .unwrap();
        assert!(out.contains("!rename <name>"));
        assert!(out.contains("!translate-me-off"));
        assert!(!out.contains("!translation"));
    }

    #[tokio::test]
    async fn help_in_main_stays_hub() {
        let store = GroupPreferencesStore::new_in_memory(0);
        store.set_sidecar("main-1", "it", "group.it".into(), "it-internal".into());
        let handler = HelpHandler::new(store, BotRole::Translation);
        let out = handler.execute(&group("!help", "main-1")).await.unwrap();
        assert!(out.contains("!translation"));
        assert!(!out.contains("!rename"));
    }
}
