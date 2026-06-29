//! Help command - displays feature menu.

use crate::commands::CommandHandler;
use crate::error::AppResult;
use async_trait::async_trait;
use signal_client::BotMessage;

pub struct HelpHandler;

impl HelpHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HelpHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandHandler for HelpHandler {
    fn trigger(&self) -> Option<&str> {
        Some("!help")
    }

    async fn execute(&self, _message: &BotMessage) -> AppResult<String> {
        Ok(r#"**Bread Coop AI** (Private & Verifiable)

**Voice:**
- !transcribe-on — auto-transcribe voice to text
- !transcribe-off — turn off auto-transcription

**Translation:**
- !translate <lang> — Quote-reply a message to translate it
- !translate-all <lang1> <lang2> — Group only: auto-translate between two languages
- !translate-off — Disable group auto-translate
- !translate-langs — List supported languages

**AI chat:**
- !ask <question> — Ask the AI anything

**Command Menus**
- !privacy — Show privacy & security menu
- !help — Show this menu"#
            .into())
    }
}
