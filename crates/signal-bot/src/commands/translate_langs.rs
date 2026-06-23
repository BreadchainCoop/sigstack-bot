//! `!translate-langs` and `!translate-langs-common` — language discovery.

use crate::commands::translate_lang::{format_language_list, ALL_LANGUAGES, COMMON_LANGUAGES};
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
    fn label(&self) -> &'static str {
        "translate_langs"
    }

    fn matches(&self, message: &BotMessage) -> bool {
        let text = message.text.trim();
        text == "!translate-langs" || text == "!translate-langs-common"
    }

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        let text = message.text.trim();
        if text == "!translate-langs-common" {
            Ok(format!(
                "**Common languages** (use code with !translate):\n\n{}",
                format_language_list(COMMON_LANGUAGES)
            ))
        } else {
            Ok(format!(
                "**Supported languages** (use code with !translate):\n\n{}",
                format_language_list(ALL_LANGUAGES)
            ))
        }
    }
}
