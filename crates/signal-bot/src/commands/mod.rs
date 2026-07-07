//! Bot command handlers.

mod ask;
mod balance;
mod chat;
mod clear;
mod deposit;
mod help;
mod manual_transcribe;
mod menu_locale;
mod models;
mod poa_confirm;
mod privacy;
mod set_language;
mod transcribe;
mod translate;
mod translate_all;
pub mod translate_lang;
mod translate_langs;
mod translate_service;
mod verify;
mod voice;

pub use ask::AskHandler;
pub use balance::BalanceHandler;
pub use chat::{ChatHandler, ToolAuthorization};
pub use clear::ClearHandler;
pub use deposit::DepositHandler;
pub use help::HelpHandler;
pub use manual_transcribe::ManualTranscribeHandler;
pub use models::ModelsHandler;
pub use poa_confirm::PoaConfirmHandler;
pub use privacy::PrivacyHandler;
pub use set_language::SetLanguageHandler;
pub use transcribe::TranscribeHandler;
pub use translate::TranslateHandler;
pub use translate_all::TranslateAllHandler;
pub use translate_langs::TranslateLangsHandler;
pub use verify::VerifyHandler;
pub use voice::VoiceHandler;

use crate::error::AppResult;
use async_trait::async_trait;
use signal_client::BotMessage;

/// Command handler trait.
#[async_trait]
pub trait CommandHandler: Send + Sync {
    /// Command trigger (e.g., "!help").
    fn trigger(&self) -> Option<&str> {
        None
    }

    /// Whether this is the default handler for non-command messages.
    fn is_default(&self) -> bool {
        false
    }

    /// Check if this handler matches the message.
    fn matches(&self, message: &BotMessage) -> bool {
        if let Some(trigger) = self.trigger() {
            message.text.starts_with(trigger)
        } else {
            self.is_default() && !message.text.starts_with('!') && !message.is_voice_note()
        }
    }

    /// Execute the command.
    async fn execute(&self, message: &BotMessage) -> AppResult<String>;

    /// When true, bot replies with a Signal quote-reply to the source message.
    fn reply_with_quote(&self) -> bool {
        false
    }

    /// When true, the handler sends its own Signal reply in `execute` (main loop skips send).
    fn handles_own_reply(&self) -> bool {
        false
    }

    /// Short name for dispatch / debug logs.
    fn label(&self) -> &'static str {
        if self.is_default() {
            "chat"
        } else {
            "command"
        }
    }
}
