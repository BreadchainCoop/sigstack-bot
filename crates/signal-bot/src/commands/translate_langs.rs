//! `!list-langs` — list supported translation languages.

use crate::commands::translate_lang::{format_language_list, ALL_LANGUAGES};
use crate::commands::CommandHandler;
use crate::error::AppResult;
use async_trait::async_trait;
use signal_bot_core::starts_with_word_any;
use signal_client::BotMessage;

pub(crate) const LIST_LANGS_COMMANDS: &[&str] = &[
    "!list-langs",
    "!list-lang",
    "!list-languages",
    "!list-language",
];

pub(crate) fn is_list_langs_command(text: &str) -> bool {
    starts_with_word_any(text, LIST_LANGS_COMMANDS)
}

pub struct TranslateLangsHandler;

impl TranslateLangsHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TranslateLangsHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandHandler for TranslateLangsHandler {
    fn trigger(&self) -> Option<&str> {
        Some("!list-langs")
    }

    fn matches(&self, message: &BotMessage) -> bool {
        is_list_langs_command(&message.text)
    }

    fn label(&self) -> &'static str {
        "translate_langs"
    }

    async fn execute(&self, _message: &BotMessage) -> AppResult<String> {
        Ok(format!(
            "**Supported languages** (use code with !translate-me-thread or !translate-me-on):\n\n{}",
            format_language_list(ALL_LANGUAGES)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_langs_exact_not_common_suffix() {
        let h = TranslateLangsHandler::new();
        let mut msg = BotMessage {
            source: "+1".into(),
            source_number: None,
            source_name: None,
            text: "!list-langs".into(),
            timestamp: 0,
            message_timestamp: 0,
            is_group: true,
            group_id: Some("g".into()),
            group_name: None,
            receiving_account: "+2".into(),
            attachments: vec![],
            quote: None,
        };
        assert!(h.matches(&msg));
        msg.text = "!list-lang".into();
        assert!(h.matches(&msg));
        msg.text = "!list-languages".into();
        assert!(h.matches(&msg));
        msg.text = "!list-language".into();
        assert!(h.matches(&msg));
        msg.text = "!List-Langs".into();
        assert!(h.matches(&msg));
        msg.text = "!list-langs-common".into();
        assert!(!h.matches(&msg));
    }

    #[tokio::test]
    async fn execute_lists_supported_languages() {
        let h = TranslateLangsHandler::new();
        let msg = BotMessage {
            source: "+1".into(),
            source_number: None,
            source_name: None,
            text: "!list-langs".into(),
            timestamp: 0,
            message_timestamp: 0,
            is_group: true,
            group_id: Some("g".into()),
            group_name: None,
            receiving_account: "+2".into(),
            attachments: vec![],
            quote: None,
        };
        let out = h.execute(&msg).await.unwrap();
        assert!(out.contains("**Supported languages**"));
        assert!(out.contains("!translate-me-thread"));
        assert!(out.contains("!translate-me-on") || out.contains("translate-me-thread"));
        assert!(out.contains("es"));
    }
}
