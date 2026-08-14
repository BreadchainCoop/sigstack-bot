//! Product menus: `!translation-threads`, `!translation-in-chat`, `!transcription`, redirects.

use crate::commands::menu_locale::{
    help_in_chat_guide, help_threads_guide, help_transcription_guide, is_exact_command,
    is_translation_in_chat_menu_command, is_translation_threads_menu_command, transcription_menu,
    translation_in_chat_menu, translation_split_redirect, translation_threads_menu,
};
use crate::commands::CommandHandler;
use crate::error::AppResult;
use async_trait::async_trait;
use signal_client::BotMessage;

/// Legacy `!translation` → points at the two product menus.
pub struct TranslationMenuHandler;

impl TranslationMenuHandler {
    pub fn new(_translate_all_enabled: bool) -> Self {
        Self
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

    async fn execute(&self, _message: &BotMessage) -> AppResult<String> {
        Ok(translation_split_redirect().into())
    }
}

pub struct TranslationThreadsMenuHandler;

impl TranslationThreadsMenuHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TranslationThreadsMenuHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandHandler for TranslationThreadsMenuHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        is_translation_threads_menu_command(&message.text)
    }

    fn label(&self) -> &'static str {
        "translation_threads_menu"
    }

    async fn execute(&self, _message: &BotMessage) -> AppResult<String> {
        Ok(translation_threads_menu().into())
    }
}

pub struct TranslationInChatMenuHandler {
    translate_all_enabled: bool,
}

impl TranslationInChatMenuHandler {
    pub fn new(translate_all_enabled: bool) -> Self {
        Self {
            translate_all_enabled,
        }
    }
}

#[async_trait]
impl CommandHandler for TranslationInChatMenuHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        is_translation_in_chat_menu_command(&message.text)
    }

    fn label(&self) -> &'static str {
        "translation_in_chat_menu"
    }

    async fn execute(&self, _message: &BotMessage) -> AppResult<String> {
        Ok(translation_in_chat_menu(self.translate_all_enabled).into())
    }
}

/// Voice command menu for `!transcription` (no pairing / invite).
pub struct TranscriptionMenuHandler;

impl TranscriptionMenuHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TranscriptionMenuHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandHandler for TranscriptionMenuHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        is_exact_command(&message.text, "!transcription")
    }

    fn label(&self) -> &'static str {
        "transcription_menu"
    }

    async fn execute(&self, _message: &BotMessage) -> AppResult<String> {
        Ok(transcription_menu().into())
    }
}

pub struct InChatMenuHandler {
    translate_all_enabled: bool,
}

impl InChatMenuHandler {
    pub fn new(translate_all_enabled: bool) -> Self {
        Self {
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

    async fn execute(&self, _message: &BotMessage) -> AppResult<String> {
        Ok(translation_in_chat_menu(self.translate_all_enabled).into())
    }
}

/// Feature guide: how Language Threads works.
pub struct HelpThreadsHandler;

impl HelpThreadsHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HelpThreadsHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandHandler for HelpThreadsHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        is_exact_command(&message.text, "!help-threads")
    }

    fn label(&self) -> &'static str {
        "help_threads"
    }

    async fn execute(&self, _message: &BotMessage) -> AppResult<String> {
        Ok(help_threads_guide().into())
    }
}

/// Feature guide: how in-chat translation works.
pub struct HelpInChatHandler;

impl HelpInChatHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HelpInChatHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandHandler for HelpInChatHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        is_exact_command(&message.text, "!help-in-chat")
    }

    fn label(&self) -> &'static str {
        "help_in_chat"
    }

    async fn execute(&self, _message: &BotMessage) -> AppResult<String> {
        Ok(help_in_chat_guide().into())
    }
}

/// Feature guide: how voice transcription works.
pub struct HelpTranscriptionHandler;

impl HelpTranscriptionHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HelpTranscriptionHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandHandler for HelpTranscriptionHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        is_exact_command(&message.text, "!help-transcription")
    }

    fn label(&self) -> &'static str {
        "help_transcription"
    }

    async fn execute(&self, _message: &BotMessage) -> AppResult<String> {
        Ok(help_transcription_guide().into())
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
            group_id: Some("g-internal".into()),
            group_name: None,
            receiving_account: "+15550001111".into(),
            attachments: vec![],
            quote: None,
        }
    }

    #[test]
    fn menus_match_exact_only() {
        let t = TranslationMenuHandler::new(true);
        assert!(t.matches(&msg("!translation")));
        assert!(!t.matches(&msg("!translation-on es en")));

        let threads = TranslationThreadsMenuHandler::new();
        assert!(threads.matches(&msg("!translation-threads")));
        assert!(threads.matches(&msg("!translate-threads")));
        assert!(threads.matches(&msg("!translate-thread")));
        assert!(threads.matches(&msg("!translation-thread")));
        assert!(!threads.matches(&msg("!translation-on es en")));

        let in_chat_prod = TranslationInChatMenuHandler::new(true);
        assert!(in_chat_prod.matches(&msg("!translation-in-chat")));
        assert!(in_chat_prod.matches(&msg("!translate-in-chat")));
        assert!(!in_chat_prod.matches(&msg("!translation-on es en")));

        let i = InChatMenuHandler::new(true);
        assert!(i.matches(&msg("!in-chat")));

        let ht = HelpThreadsHandler::new();
        assert!(ht.matches(&msg("!help-threads")));
        assert!(!ht.matches(&msg("!help")));

        let hi = HelpInChatHandler::new();
        assert!(hi.matches(&msg("!help-in-chat")));
        assert!(!hi.matches(&msg("!help")));

        let htr = HelpTranscriptionHandler::new();
        assert!(htr.matches(&msg("!help-transcription")));
        assert!(!htr.matches(&msg("!help")));
        assert!(!htr.matches(&msg("!transcription")));

        let m = TranscriptionMenuHandler::new();
        assert!(m.matches(&msg("!transcription")));
    }

    #[tokio::test]
    async fn in_chat_typo_aliases_match_canonical_menu() {
        let canonical = TranslationInChatMenuHandler::new(true);
        let expected = canonical
            .execute(&msg("!translation-in-chat"))
            .await
            .unwrap();
        for typo in ["!translate-in-chat"] {
            let got = canonical.execute(&msg(typo)).await.unwrap();
            assert_eq!(got, expected);
        }
    }

    #[tokio::test]
    async fn threads_typo_aliases_match_canonical_menu() {
        let handler = TranslationThreadsMenuHandler::new();
        let expected = handler.execute(&msg("!translation-threads")).await.unwrap();
        for typo in [
            "!translate-threads",
            "!translate-thread",
            "!translation-thread",
        ] {
            let got = handler.execute(&msg(typo)).await.unwrap();
            assert_eq!(got, expected);
        }
    }

    #[tokio::test]
    async fn in_chat_alias_matches_in_chat_menu() {
        let product = TranslationInChatMenuHandler::new(true);
        let alias = InChatMenuHandler::new(true);
        let via_product = product.execute(&msg("!translation-in-chat")).await.unwrap();
        let via_alias = alias.execute(&msg("!in-chat")).await.unwrap();
        assert_eq!(via_product, via_alias);
        assert!(via_product.contains("!translate-all-on"));
        assert!(via_product.contains("!translate-me-on"));
        assert!(!via_product.contains("!translate-me-thread"));
    }

    #[tokio::test]
    async fn translation_redirect_names_both_menus() {
        let translation = TranslationMenuHandler::new(true);
        let out = translation.execute(&msg("!translation")).await.unwrap();
        assert!(out.contains("!translation-threads"));
        assert!(out.contains("!translation-in-chat"));
    }

    #[tokio::test]
    async fn feature_guides_return_use_case_copy() {
        let threads = HelpThreadsHandler::new()
            .execute(&msg("!help-threads"))
            .await
            .unwrap();
        assert!(threads.contains("sidecar"));
        assert!(threads.contains("!translate-me-thread"));

        let in_chat = HelpInChatHandler::new()
            .execute(&msg("!help-in-chat"))
            .await
            .unwrap();
        assert!(in_chat.contains("quote"));
        assert!(in_chat.contains("!translate-all-on"));

        let transcription = HelpTranscriptionHandler::new()
            .execute(&msg("!help-transcription"))
            .await
            .unwrap();
        assert!(transcription.contains("Whisper"));
        assert!(transcription.contains("!transcribe"));
    }

    #[tokio::test]
    async fn transcription_menu_returns_voice_commands() {
        let out = TranscriptionMenuHandler::new()
            .execute(&msg("!transcription"))
            .await
            .unwrap();
        assert!(out.contains("!transcribe-on"));
        assert!(out.contains("!transcribe-off"));
        assert!(out.contains("!transcribe"));
        assert!(out.contains("!help-transcription"));
        assert!(!out.contains("!privacy"));
        assert!(!out.contains("!translation-threads"));
    }
}
