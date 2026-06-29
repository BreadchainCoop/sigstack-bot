//! `!translate-langs` — list supported translation languages.

use crate::commands::translate_lang::{format_language_list, ALL_LANGUAGES};
use crate::commands::CommandHandler;
use crate::error::AppResult;
use async_trait::async_trait;
use signal_client::BotMessage;

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
        Some("!translate-langs")
    }

    fn label(&self) -> &'static str {
        "translate_langs"
    }

    async fn execute(&self, _message: &BotMessage) -> AppResult<String> {
        Ok(format!(
            "**Supported languages** (use code with !translate):\n\n{}",
            format_language_list(ALL_LANGUAGES)
        ))
    }
}
