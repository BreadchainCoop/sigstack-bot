//! Nested product menus: `!translation`, `!transcription`, `!in-chat`, `!parallel`.

use crate::commands::menu_locale::{
    in_chat_menu, is_exact_command, menu_language_for_message, parallel_menu,
    transcription_unavailable, translation_products_menu,
};
use crate::commands::CommandHandler;
use crate::error::AppResult;
use crate::group_preferences_store::GroupPreferencesStore;
use async_trait::async_trait;
use signal_client::BotMessage;
use std::sync::Arc;

pub struct TranslationMenuHandler {
    group_prefs: Arc<GroupPreferencesStore>,
}

impl TranslationMenuHandler {
    pub fn new(group_prefs: Arc<GroupPreferencesStore>) -> Self {
        Self { group_prefs }
    }
}

#[async_trait]
impl CommandHandler for TranslationMenuHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        is_exact_command(&message.text, "!translation")
    }

    fn label(&self) -> &'static str {
        "translation_menu"
    }

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        let language = menu_language_for_message(message, &self.group_prefs);
        Ok(translation_products_menu(language).into())
    }
}

pub struct TranscriptionStubHandler {
    group_prefs: Arc<GroupPreferencesStore>,
}

impl TranscriptionStubHandler {
    pub fn new(group_prefs: Arc<GroupPreferencesStore>) -> Self {
        Self { group_prefs }
    }
}

#[async_trait]
impl CommandHandler for TranscriptionStubHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        is_exact_command(&message.text, "!transcription")
    }

    fn label(&self) -> &'static str {
        "transcription_stub"
    }

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        let language = menu_language_for_message(message, &self.group_prefs);
        Ok(transcription_unavailable(language).into())
    }
}

pub struct InChatMenuHandler {
    group_prefs: Arc<GroupPreferencesStore>,
    translate_all_enabled: bool,
}

impl InChatMenuHandler {
    pub fn new(group_prefs: Arc<GroupPreferencesStore>, translate_all_enabled: bool) -> Self {
        Self {
            group_prefs,
            translate_all_enabled,
        }
    }
}

#[async_trait]
impl CommandHandler for InChatMenuHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        is_exact_command(&message.text, "!in-chat")
    }

    fn label(&self) -> &'static str {
        "in_chat_menu"
    }

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        let language = menu_language_for_message(message, &self.group_prefs);
        Ok(in_chat_menu(language, self.translate_all_enabled).into())
    }
}

pub struct ParallelMenuHandler {
    group_prefs: Arc<GroupPreferencesStore>,
}

impl ParallelMenuHandler {
    pub fn new(group_prefs: Arc<GroupPreferencesStore>) -> Self {
        Self { group_prefs }
    }
}

#[async_trait]
impl CommandHandler for ParallelMenuHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        is_exact_command(&message.text, "!parallel")
    }

    fn label(&self) -> &'static str {
        "parallel_menu"
    }

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        let language = menu_language_for_message(message, &self.group_prefs);
        Ok(parallel_menu(language).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(text: &str) -> BotMessage {
        BotMessage {
            source: "+1".into(),
            source_number: None,
            source_name: None,
            text: text.into(),
            timestamp: 0,
            message_timestamp: 0,
            is_group: true,
            group_id: Some("g".into()),
            group_name: None,
            receiving_account: "+2".into(),
            attachments: vec![],
            quote: None,
        }
    }

    #[test]
    fn menus_match_exact_only() {
        let store = GroupPreferencesStore::new_in_memory(0);
        let t = TranslationMenuHandler::new(store.clone());
        assert!(t.matches(&msg("!translation")));
        assert!(!t.matches(&msg("!translation-on es en")));

        let p = ParallelMenuHandler::new(store.clone());
        assert!(p.matches(&msg("!parallel")));
        assert!(!p.matches(&msg("!parallel-on en es")));

        let i = InChatMenuHandler::new(store.clone(), true);
        assert!(i.matches(&msg("!in-chat")));

        let s = TranscriptionStubHandler::new(store);
        assert!(s.matches(&msg("!transcription")));
    }

    #[tokio::test]
    async fn transcription_stub_offers_translation() {
        let store = GroupPreferencesStore::new_in_memory(0);
        let handler = TranscriptionStubHandler::new(store);
        let out = handler.execute(&msg("!transcription")).await.unwrap();
        assert!(out.contains("!translation"));
        assert!(out.to_lowercase().contains("unavailable"));
    }
}
