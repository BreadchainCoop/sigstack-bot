//! Help / info commands — compact menu vs info menu.

use crate::commands::menu_locale::{
    help_menu, info_menu, is_exact_command, thread_help_menu, thread_info_menu,
};
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
    fn matches(&self, message: &BotMessage) -> bool {
        // Exact match so !help-threads / !help-in-chat are not swallowed.
        is_exact_command(&message.text, "!help")
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

/// Same menus as [`HelpHandler`], with per-command explanations and blank-line breaks.
pub struct InfoHandler {
    group_prefs: Arc<GroupPreferencesStore>,
    role: BotRole,
}

impl InfoHandler {
    pub fn new(group_prefs: Arc<GroupPreferencesStore>, role: BotRole) -> Self {
        Self { group_prefs, role }
    }
}

#[async_trait]
impl CommandHandler for InfoHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        is_exact_command(&message.text, "!info")
    }

    fn label(&self) -> &'static str {
        "info"
    }

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        if self.role == BotRole::Translation {
            if let Some(group_id) = message.group_id.as_deref() {
                if self.group_prefs.lookup_sidecar(group_id).is_some() {
                    return Ok(thread_info_menu().into());
                }
            }
        }
        Ok(info_menu(self.role).into())
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
        assert!(!transcription.matches(&dm("!help-threads")));
        assert!(!transcription.matches(&dm("!help-in-chat")));
        let t = transcription.execute(&dm("!help")).await.unwrap();
        assert!(t.contains("!transcribe"));
        assert!(t.contains("!help-transcription"));
        assert!(!t.contains("!privacy"));
        assert!(!t.contains("!translate-me-on"));

        let t = translation.execute(&dm("!help")).await.unwrap();
        assert!(t.contains("!translation-threads"));
        assert!(t.contains("!translation-in-chat"));
        assert!(t.contains("!transcription"));
        assert!(t.contains("!privacy"));
        assert!(t.contains("!info"));
        assert!(!t.contains("Voice notes in this chat"));
        assert!(!t.contains("!set-en"));
    }

    #[tokio::test]
    async fn info_returns_described_hub() {
        let store = GroupPreferencesStore::new_in_memory(0);
        let handler = InfoHandler::new(store, BotRole::Translation);
        assert!(handler.matches(&dm("!info")));
        let out = handler.execute(&dm("!info")).await.unwrap();
        assert!(out.contains("!translation-threads\n  "));
        assert!(out.contains("\n\n!privacy"));
        assert!(out.contains("Compact command list"));
        assert!(out.contains("!privacy\n  "));
        assert!(out.contains("TEE"));
        assert!(!out.contains("!verify <challenge>"));
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
        assert!(out.contains("!leave"));
        assert!(!out.contains("!translation-threads"));
    }

    #[tokio::test]
    async fn info_in_sidecar_returns_thread_info() {
        let store = GroupPreferencesStore::new_in_memory(0);
        store.set_sidecar("main-1", "it", "group.it".into(), "it-internal".into());
        let handler = InfoHandler::new(store, BotRole::Translation);
        let out = handler
            .execute(&group("!info", "it-internal"))
            .await
            .unwrap();
        assert!(out.contains("!rename <name>\n  "));
        assert!(out.contains("!leave\n  "));
        assert!(!out.contains("!translation-threads"));
    }

    #[tokio::test]
    async fn help_in_main_stays_hub() {
        let store = GroupPreferencesStore::new_in_memory(0);
        store.set_sidecar("main-1", "it", "group.it".into(), "it-internal".into());
        let handler = HelpHandler::new(store, BotRole::Translation);
        let out = handler.execute(&group("!help", "main-1")).await.unwrap();
        assert!(out.contains("!translation-threads"));
        assert!(!out.contains("!rename"));
    }
}
