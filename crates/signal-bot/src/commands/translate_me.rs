//! `!translate-me on <lang>` / `!translate-me off` — per-user opt-in translation.
//!
//! Unlike group-wide `!translate-on`, this only translates the messages of the
//! individual user who opted in. Each opted-in user picks a single target
//! language; their own outgoing group messages are translated into it and
//! posted back to the group. Everyone else's messages are left untouched.

use crate::commands::translate_lang::resolve_language;
use crate::commands::translate_service::{
    detect_text_language, format_text_auto_translation, near_ai_translate,
};
use crate::commands::CommandHandler;
use crate::error::AppResult;
use crate::group_preferences_store::GroupPreferencesStore;
use async_trait::async_trait;
use near_ai_client::NearAiClient;
use signal_client::{BotMessage, SignalClient};
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

const COMMAND_PREFIXES: &[&str] = &["!translate-me", "!translation-me"];

const GROUP_ONLY_MSG: &str = "!translate-me is only available in group chats";
const USAGE_MSG: &str = "Usage: !translate-me on <lang> (e.g. !translate-me on en), or !translate-me off";

pub struct TranslateMeHandler {
    store: Arc<GroupPreferencesStore>,
    near_ai: Arc<NearAiClient>,
    signal: Arc<SignalClient>,
}

impl TranslateMeHandler {
    pub fn new(
        store: Arc<GroupPreferencesStore>,
        near_ai: Arc<NearAiClient>,
        signal: Arc<SignalClient>,
    ) -> Self {
        Self {
            store,
            near_ai,
            signal,
        }
    }

    fn is_command(text: &str) -> bool {
        let text = text.trim();
        COMMAND_PREFIXES.iter().any(|prefix| text == *prefix || text.starts_with(&format!("{prefix} ")))
    }

    fn is_text_intercept(message: &BotMessage) -> bool {
        let text = message.text.trim();
        message.group_id.is_some()
            && !message.is_voice_note()
            && !text.is_empty()
            && !text.starts_with('!')
    }

    /// The arguments following the command prefix (e.g. `"on en"`).
    fn command_args(text: &str) -> &str {
        let text = text.trim();
        COMMAND_PREFIXES
            .iter()
            .find_map(|prefix| text.strip_prefix(prefix))
            .map(str::trim)
            .unwrap_or("")
    }

    async fn handle_command(&self, message: &BotMessage) -> AppResult<String> {
        let group_id = match message.group_id.as_deref() {
            Some(id) => id,
            None => return Ok(GROUP_ONLY_MSG.into()),
        };

        let args = Self::command_args(&message.text);
        let mut parts = args.split_whitespace();

        match parts.next().map(str::to_lowercase).as_deref() {
            Some("off") => {
                if self.store.clear_user_translate(group_id, &message.source) {
                    info!(group_id, source = %message.source, "translate-me disabled");
                    Ok("Translation off: your messages will no longer be translated.".into())
                } else {
                    Ok("Translation was not active for you in this chat.".into())
                }
            }
            // `!translate-me on <lang>` or the convenience form `!translate-me <lang>`.
            Some("on") => self.enable(group_id, message, parts.next()),
            Some(token) if resolve_language(token).is_some() => {
                self.enable(group_id, message, Some(token))
            }
            _ => Ok(USAGE_MSG.into()),
        }
    }

    fn enable(
        &self,
        group_id: &str,
        message: &BotMessage,
        lang_token: Option<&str>,
    ) -> AppResult<String> {
        let token = match lang_token {
            Some(t) if !t.is_empty() => t,
            _ => return Ok(USAGE_MSG.into()),
        };

        let lang = match resolve_language(token) {
            Some(l) => l,
            None => {
                return Ok(format!(
                    "Unknown language: {token}. Use !list-langs for supported codes."
                ));
            }
        };

        self.store
            .set_user_translate(group_id, &message.source, lang.code.to_string());
        info!(
            group_id,
            source = %message.source,
            target = lang.code,
            "translate-me enabled"
        );
        Ok(format!(
            "Translation on: your messages will be translated to {} {}.",
            lang.flag, lang.name
        ))
    }

    async fn handle_text_intercept(&self, message: &BotMessage) -> AppResult<()> {
        let group_id = match message.group_id.as_deref() {
            Some(id) => id,
            None => return Ok(()),
        };

        let target_code = match self.store.get_user_translate(group_id, &message.source) {
            Some(code) => code,
            None => return Ok(()),
        };
        let target = match resolve_language(&target_code) {
            Some(lang) => lang,
            None => return Ok(()),
        };

        let text = message.text.trim();

        // Skip if the message already looks like the target language.
        if let Some(detected) = detect_text_language(text) {
            if detected == target.code {
                debug!(
                    group_id,
                    target = target.code,
                    "translate-me skipped (message already in target language)"
                );
                return Ok(());
            }
        }

        if !self.store.allow_message(group_id) {
            warn!(group_id, "translate-me rate limited — skipping text message");
            return Ok(());
        }

        let translation = match near_ai_translate(&self.near_ai, text, target).await {
            Ok(t) => t,
            Err(e) => {
                warn!("translate-me translation failed: {}", e);
                self.signal
                    .reply_quoted(message, "Could not translate. Try again later.", None)
                    .await?;
                return Ok(());
            }
        };

        let body = format_text_auto_translation(target, &translation);
        self.signal.reply_quoted(message, &body, None).await?;
        info!(
            group_id,
            source = %message.source,
            target = target.code,
            translation_chars = translation.len(),
            "translate-me text translated"
        );
        Ok(())
    }
}

#[async_trait]
impl CommandHandler for TranslateMeHandler {
    fn label(&self) -> &'static str {
        "translate_me"
    }

    fn matches(&self, message: &BotMessage) -> bool {
        if Self::is_command(&message.text) {
            return true;
        }
        if Self::is_text_intercept(message) {
            if let Some(gid) = &message.group_id {
                return self.store.is_user_translate_active(gid, &message.source);
            }
        }
        false
    }

    fn handles_own_reply(&self) -> bool {
        true
    }

    #[instrument(skip(self, message), fields(source = %message.source, is_group = message.is_group))]
    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        if Self::is_command(&message.text) {
            let response = self.handle_command(message).await?;
            self.signal.reply(message, &response).await?;
            return Ok(String::new());
        }

        self.handle_text_intercept(message).await?;
        Ok(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_handler() -> TranslateMeHandler {
        TranslateMeHandler::new(
            GroupPreferencesStore::new_in_memory(30),
            Arc::new(
                NearAiClient::new(
                    "key",
                    "http://localhost",
                    "model",
                    std::time::Duration::from_secs(5),
                )
                .unwrap(),
            ),
            Arc::new(SignalClient::new("http://localhost").unwrap()),
        )
    }

    fn group_msg(source: &str, text: &str) -> BotMessage {
        BotMessage {
            source: source.into(),
            text: text.into(),
            timestamp: 0,
            message_timestamp: 0,
            is_group: true,
            group_id: Some("gid".into()),
            receiving_account: "+bot".into(),
            attachments: vec![],
            quote: None,
        }
    }

    #[test]
    fn recognizes_commands() {
        assert!(TranslateMeHandler::is_command("!translate-me on en"));
        assert!(TranslateMeHandler::is_command("!translate-me off"));
        assert!(TranslateMeHandler::is_command("!translate-me"));
        assert!(TranslateMeHandler::is_command("!translation-me on es"));
        // Must not swallow the group-wide command or bare !translate.
        assert!(!TranslateMeHandler::is_command("!translate-on es en"));
        assert!(!TranslateMeHandler::is_command("!translate es"));
    }

    #[test]
    fn command_args_extracted() {
        assert_eq!(TranslateMeHandler::command_args("!translate-me on en"), "on en");
        assert_eq!(TranslateMeHandler::command_args("!translate-me off"), "off");
        assert_eq!(TranslateMeHandler::command_args("!translate-me"), "");
    }

    #[test]
    fn intercept_only_matches_opted_in_user() {
        let handler = test_handler();

        // Not opted in yet — no intercept.
        assert!(!handler.matches(&group_msg("+alice", "Hola a todos")));

        handler
            .store
            .set_user_translate("gid", "+alice", "en".into());

        // Alice opted in — her plain text is intercepted.
        assert!(handler.matches(&group_msg("+alice", "Hola a todos")));
        // Bob did not opt in — his messages are left alone.
        assert!(!handler.matches(&group_msg("+bob", "Hola a todos")));
        // Commands are always claimed regardless of opt-in state.
        assert!(handler.matches(&group_msg("+bob", "!translate-me on es")));
        // But a bare `!` message from Alice is not treated as translatable text.
        assert!(!handler.matches(&group_msg("+alice", "!help")));
    }

    #[tokio::test]
    async fn enable_and_disable_roundtrip() {
        let handler = test_handler();

        let on = handler
            .handle_command(&group_msg("+alice", "!translate-me on en"))
            .await
            .unwrap();
        assert!(on.contains("English"));
        assert!(handler.store.is_user_translate_active("gid", "+alice"));
        assert_eq!(
            handler.store.get_user_translate("gid", "+alice").as_deref(),
            Some("en")
        );

        let off = handler
            .handle_command(&group_msg("+alice", "!translate-me off"))
            .await
            .unwrap();
        assert!(off.to_lowercase().contains("off"));
        assert!(!handler.store.is_user_translate_active("gid", "+alice"));
    }

    #[tokio::test]
    async fn convenience_form_without_on_keyword() {
        let handler = test_handler();
        let reply = handler
            .handle_command(&group_msg("+alice", "!translate-me es"))
            .await
            .unwrap();
        assert!(reply.contains("Spanish"));
        assert_eq!(
            handler.store.get_user_translate("gid", "+alice").as_deref(),
            Some("es")
        );
    }

    #[tokio::test]
    async fn unknown_language_reports_error() {
        let handler = test_handler();
        let reply = handler
            .handle_command(&group_msg("+alice", "!translate-me on klingon"))
            .await
            .unwrap();
        assert!(reply.contains("Unknown language"));
        assert!(!handler.store.is_user_translate_active("gid", "+alice"));
    }

    #[tokio::test]
    async fn bare_command_shows_usage() {
        let handler = test_handler();
        let reply = handler
            .handle_command(&group_msg("+alice", "!translate-me"))
            .await
            .unwrap();
        assert!(reply.contains("Usage"));
    }

    #[tokio::test]
    async fn group_only() {
        let handler = test_handler();
        let mut msg = group_msg("+alice", "!translate-me on en");
        msg.group_id = None;
        msg.is_group = false;
        let reply = handler.handle_command(&msg).await.unwrap();
        assert_eq!(reply, GROUP_ONLY_MSG);
    }
}
