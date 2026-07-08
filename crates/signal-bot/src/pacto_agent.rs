//! Inbound Pacto DM agent — gives Pacto users the same DM experience Signal
//! users get.
//!
//! A background task consumes decrypted DMs delivered by the pacto-bot-api
//! daemon, runs each one through the *same* command/AI-chat handlers the Signal
//! side uses (via a synthetic [`BotMessage`]), and replies over Pacto.
//!
//! ## Parity ceiling
//!
//! The daemon only delivers `dm_received` events — it does not expose inbound
//! MLS group messages or audio attachments to handlers. So Pacto parity covers
//! the full **DM** experience (AI chat with tools, `!verify`, `!clear`,
//! `!models`, `!help`, `!privacy`, `!list-langs`, and AI-driven translation),
//! but **group translation and voice transcription are not possible** until the
//! daemon gains inbound group/attachment delivery.

use crate::commands::{
    ChatHandler, ClearHandler, CommandHandler, ModelsHandler, ProgressSink, SignalRelayHandler,
    TranslateLangsHandler, VerifyHandler,
};
use async_trait::async_trait;
use conversation_store::ConversationStore;
use dstack_client::DstackClient;
use near_ai_client::NearAiClient;
use pacto_client::{InboundDm, PactoAgent, PactoClient};
use signal_client::{BotMessage, SignalClient};
use std::sync::Arc;
use tokio::sync::mpsc;
use tools::ToolRegistry;
use tracing::{debug, error, info, warn};

/// Everything the agent loop needs to construct the Pacto DM handler set.
pub struct PactoAgentDeps {
    pub near_ai: Arc<NearAiClient>,
    pub conversations: Arc<ConversationStore>,
    pub tool_registry: Arc<ToolRegistry>,
    pub dstack: Arc<DstackClient>,
    /// Outbound request/response client, reused to deliver progress pings.
    pub pacto_client: Arc<PactoClient>,
    pub system_prompt: String,
    pub max_tool_calls: usize,
    pub signal_username: Option<String>,
    pub github_repo: Option<String>,
    /// Optional `!signal` relay (Pacto user → Signal user). `None` disables it.
    pub signal_relay: Option<SignalRelay>,
}

/// Configuration for the `!signal` relay handler.
pub struct SignalRelay {
    pub signal: Arc<SignalClient>,
    /// The bot's own Signal number (E.164) to send from.
    pub from_number: String,
    /// Permitted recipient numbers, or `["*"]` for any.
    pub allowlist: Vec<String>,
}

/// Spawn the inbound Pacto DM loop. Returns immediately; the loop runs until the
/// inbound channel closes (daemon supervisor gives up / process shuts down).
pub fn spawn(inbound: mpsc::Receiver<InboundDm>, agent: Arc<PactoAgent>, deps: PactoAgentDeps) {
    let handlers = build_handlers(deps);
    tokio::spawn(run(inbound, agent, handlers));
}

fn build_handlers(deps: PactoAgentDeps) -> Vec<Box<dyn CommandHandler>> {
    let progress: Arc<dyn ProgressSink> = Arc::new(PactoProgress {
        pacto: deps.pacto_client,
    });
    let chat = ChatHandler::new(
        deps.near_ai.clone(),
        deps.conversations.clone(),
        progress,
        deps.tool_registry,
        deps.system_prompt,
        deps.max_tool_calls,
        deps.signal_username,
        deps.github_repo,
    );

    // Command handlers first, AI chat (default) last — same precedence as the
    // Signal dispatch loop. Only transport-clean handlers are reused; voice,
    // group-translate, and Signal-quote `!translate` are Signal-specific.
    let relay_enabled = deps.signal_relay.is_some();
    let mut handlers: Vec<Box<dyn CommandHandler>> = vec![
        Box::new(VerifyHandler::new(deps.dstack)),
        Box::new(ClearHandler::new(deps.conversations)),
        Box::new(ModelsHandler::new(deps.near_ai)),
        Box::new(PactoHelpHandler { relay_enabled }),
        Box::new(PactoPrivacyHandler),
        Box::new(TranslateLangsHandler::new()),
    ];

    // Optional: let Pacto users DM Signal users via `!signal` (allowlist-gated).
    if let Some(relay) = deps.signal_relay {
        handlers.push(Box::new(SignalRelayHandler::new(
            relay.signal,
            relay.from_number,
            relay.allowlist,
        )));
        info!("Pacto→Signal relay enabled: !signal");
    }

    // AI chat is the default (matches non-command free text), so it goes last.
    handlers.push(Box::new(chat));
    handlers
}

async fn run(
    mut inbound: mpsc::Receiver<InboundDm>,
    agent: Arc<PactoAgent>,
    handlers: Vec<Box<dyn CommandHandler>>,
) {
    info!("Pacto DM agent ready ({} handlers)", handlers.len());
    while let Some(dm) = inbound.recv().await {
        let message = to_bot_message(&dm);
        debug!(author = %shorten(&dm.author), "Pacto DM received");

        match dispatch(&handlers, &message).await {
            Some(reply) => {
                if let Err(e) = agent.reply(&dm.event_id, &reply).await {
                    warn!(error = %e, "Failed to send Pacto reply");
                }
            }
            None => {
                // Nothing to say (empty text or unknown !command): terminate the
                // event cleanly so the daemon doesn't consider it unhandled.
                let _ = agent.finish(&dm.event_id, "ignore").await;
            }
        }
    }
    info!("Pacto DM agent stopped");
}

/// Find the first matching handler and run it, mirroring the Signal main loop.
async fn dispatch(handlers: &[Box<dyn CommandHandler>], message: &BotMessage) -> Option<String> {
    let handler = handlers.iter().find(|h| h.matches(message))?;
    match handler.execute(message).await {
        Ok(reply) => Some(reply),
        Err(e) => {
            error!(handler = handler.label(), error = %e, "Pacto handler error");
            Some("Sorry, something went wrong.".to_string())
        }
    }
}

/// Present a Pacto DM as a Signal-shaped [`BotMessage`] so existing handlers
/// work unchanged. The author's pubkey is the `source`, so `reply_target()`
/// (and thus the conversation key) is per-sender.
fn to_bot_message(dm: &InboundDm) -> BotMessage {
    BotMessage {
        source: dm.author.clone(),
        text: dm.content.clone(),
        timestamp: dm.timestamp,
        message_timestamp: dm.timestamp,
        is_group: false,
        group_id: None,
        receiving_account: dm.bot_id.clone(),
        attachments: Vec::new(),
        quote: None,
    }
}

fn shorten(key: &str) -> String {
    if key.len() > 12 {
        format!("{}…", &key[..12])
    } else {
        key.to_string()
    }
}

/// Progress sink that delivers "🔧 Using ..." pings as plain Pacto DMs via the
/// request/response client (the final reply goes over the agent connection).
struct PactoProgress {
    pacto: Arc<PactoClient>,
}

#[async_trait]
impl ProgressSink for PactoProgress {
    async fn notify(&self, message: &BotMessage, text: &str) {
        if let Err(e) = self.pacto.send_dm(&message.source, text).await {
            debug!(error = %e, "Failed to send Pacto progress ping");
        }
    }
}

/// Pacto-accurate `!help` (omits Signal-only voice/group features).
struct PactoHelpHandler {
    /// Whether the `!signal` relay is active (so help only lists it when usable).
    relay_enabled: bool,
}

#[async_trait]
impl CommandHandler for PactoHelpHandler {
    fn trigger(&self) -> Option<&str> {
        Some("!help")
    }

    fn label(&self) -> &'static str {
        "pacto-help"
    }

    async fn execute(&self, _message: &BotMessage) -> crate::error::AppResult<String> {
        let mut help = PACTO_HELP.to_string();
        if self.relay_enabled {
            help.push_str("\n- !signal <+number> <message> — DM a Signal user");
        }
        Ok(help)
    }
}

/// Pacto-accurate `!privacy`.
struct PactoPrivacyHandler;

#[async_trait]
impl CommandHandler for PactoPrivacyHandler {
    fn trigger(&self) -> Option<&str> {
        Some("!privacy")
    }

    fn label(&self) -> &'static str {
        "pacto-privacy"
    }

    async fn execute(&self, _message: &BotMessage) -> crate::error::AppResult<String> {
        Ok(PACTO_PRIVACY.to_string())
    }
}

const PACTO_HELP: &str = r#"**Bread Coop AI on Pacto** (Private & Verifiable)

**AI chat:**
- Just message me — I reply with private AI inference
- I can search the web, do math, and check the weather
- Ask me to translate, e.g. "translate 'good morning' to Spanish"

**Commands:**
- !verify <challenge> — cryptographic TEE attestation
- !models — list available AI models
- !clear — clear our conversation history
- !list-langs — supported translation languages
- !privacy — privacy & security details
- !help — this menu"#;

const PACTO_PRIVACY: &str = r#"**Bread Coop AI on Pacto** (Private & Verifiable)

Your messages are end-to-end encrypted over Nostr (NIP-17 gift wraps) and
decrypted only inside an Intel TDX Trusted Execution Environment. The bot's
Nostr key and your plaintext never leave the TEE.

AI inference runs on NEAR AI Cloud's private GPU TEE. Neither the bot operator
nor the AI provider can read your messages.

Send `!verify <challenge>` for a fresh hardware attestation proving this bot
runs in a genuine TEE."#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn test_handlers() -> Vec<Box<dyn CommandHandler>> {
        build_handlers(PactoAgentDeps {
            near_ai: Arc::new(
                NearAiClient::new("k", "http://localhost", "m", Duration::from_secs(5)).unwrap(),
            ),
            conversations: Arc::new(ConversationStore::new(50, Duration::from_secs(3600))),
            tool_registry: Arc::new(ToolRegistry::new()),
            dstack: Arc::new(DstackClient::new("/tmp/nonexistent-dstack.sock")),
            pacto_client: Arc::new(PactoClient::new(
                "/tmp/nonexistent-pacto.sock",
                "test-bot",
                Duration::from_secs(5),
            )),
            system_prompt: String::new(),
            max_tool_calls: 5,
            signal_username: None,
            github_repo: None,
            signal_relay: None,
        })
    }

    fn dm(text: &str) -> InboundDm {
        InboundDm {
            bot_id: "test-bot".into(),
            event_id: "evt".into(),
            chat_id: Some("npub1author".into()),
            author: "npub1author".into(),
            content: text.into(),
            rumor_id: "rumor".into(),
            timestamp: 0,
        }
    }

    #[test]
    fn bot_message_is_a_dm_keyed_by_author() {
        let m = to_bot_message(&dm("hi"));
        assert_eq!(m.source, "npub1author");
        assert!(!m.is_group);
        assert_eq!(m.group_id, None);
        // Conversation key is the sender, so history is per-Pacto-user.
        assert_eq!(m.reply_target(), "npub1author");
        assert_eq!(m.receiving_account, "test-bot");
    }

    #[tokio::test]
    async fn help_routes_to_pacto_specific_menu() {
        let handlers = test_handlers();
        let reply = dispatch(&handlers, &to_bot_message(&dm("!help")))
            .await
            .expect("help should produce a reply");
        assert!(reply.contains("Bread Coop AI on Pacto"));
        // Must not advertise Signal-only features Pacto can't offer.
        assert!(!reply.contains("!transcribe"));
        assert!(!reply.contains("!translate-on"));
    }

    #[tokio::test]
    async fn privacy_and_list_langs_route() {
        let handlers = test_handlers();
        let privacy = dispatch(&handlers, &to_bot_message(&dm("!privacy")))
            .await
            .expect("privacy reply");
        assert!(privacy.contains("TDX"));
        assert!(
            dispatch(&handlers, &to_bot_message(&dm("!list-langs")))
                .await
                .is_some()
        );
    }

    #[tokio::test]
    async fn empty_message_is_ignored() {
        let handlers = test_handlers();
        assert!(
            dispatch(&handlers, &to_bot_message(&dm("   ")))
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn unknown_command_is_ignored() {
        let handlers = test_handlers();
        // A "!"-prefixed token matches no command and is excluded from chat.
        assert!(
            dispatch(&handlers, &to_bot_message(&dm("!nope")))
                .await
                .is_none()
        );
    }
}
