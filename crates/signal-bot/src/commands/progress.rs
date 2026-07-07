//! Transport-agnostic progress notifications.
//!
//! The chat handler emits best-effort progress pings (e.g. "🔧 Using
//! web_search...") while it runs tools. On Signal these go out as a reply; on
//! Pacto they go out as a DM. `ProgressSink` abstracts that so the same chat
//! logic drives both transports.

use async_trait::async_trait;
use signal_client::{BotMessage, SignalClient};
use tracing::warn;

/// Sends best-effort, non-final progress messages back to the user.
#[async_trait]
pub trait ProgressSink: Send + Sync {
    /// Deliver an intermediate progress message. Failures are non-fatal.
    async fn notify(&self, message: &BotMessage, text: &str);
}

#[async_trait]
impl ProgressSink for SignalClient {
    async fn notify(&self, message: &BotMessage, text: &str) {
        if let Err(e) = self.reply(message, text).await {
            warn!("Failed to send progress message: {}", e);
        }
    }
}
