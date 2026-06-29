//! `!privacy` — privacy, security, and TEE commands menu.

use crate::commands::CommandHandler;
use crate::error::AppResult;
use async_trait::async_trait;
use signal_client::BotMessage;

pub struct PrivacyHandler;

impl PrivacyHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PrivacyHandler {
    fn default() -> Self {
        Self::new()
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

    async fn execute(&self, _message: &BotMessage) -> AppResult<String> {
        Ok(r#"**Bread Coop AI** (Private & Verifiable)

**TEE Commands:**
- !verify <challenge> - Get TEE attestation with your challenge
- !clear - Clear conversation history
- !models - List available AI models

**Command Menus**
- !privacy - Show this message
- !help - Show feature menu

**Verification:**
`!verify my-random-text` to get cryptographic proof this bot runs in a TEE. Your challenge is embedded in the TDX quote, proving the attestation was generated fresh for you.

**Privacy:**
Your messages are end-to-end encrypted via Signal, processed in a verified TEE (Intel TDX), and sent to NEAR AI Cloud's private inference (NVIDIA GPU TEE).

Voice transcription runs locally in the TEE (Whisper). Translation uses NEAR AI on text only.

Neither the bot operator nor NEAR AI can read your messages."#
            .into())
    }
}
