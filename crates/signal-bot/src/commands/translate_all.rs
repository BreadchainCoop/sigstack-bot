//! `!translate-on` / `!translate-off` — group auto-translate mode.

use crate::commands::translate_lang::resolve_language;
use crate::commands::translate_service::{
    format_text_auto_translation, near_ai_translate, target_for_message_text,
};
use crate::commands::CommandHandler;
use crate::error::AppResult;
use crate::group_preferences_store::{GroupPreferencesStore, GroupTranslateMode};
use async_trait::async_trait;
use near_ai_client::NearAiClient;
use signal_client::{BotMessage, SignalClient};
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

const TRANSLATE_ON_PREFIXES: &[&str] = &["!translate-on", "!translation-on"];
const TRANSLATE_OFF_COMMANDS: &[&str] = &["!translate-off", "!translation-off"];

const BARE_COMMAND_MSG: &str =
    "Please specify two languages. Example: !translate-on es en";
const GROUP_ONLY_MSG: &str = "!translate-on is only available in group chats";

/// Whether the message is `!translate-on` / `!translation-on` or the off variant.
pub(crate) fn is_translate_on_or_off_command(text: &str) -> bool {
    let text = text.trim();
    TRANSLATE_ON_PREFIXES
        .iter()
        .any(|prefix| text.starts_with(prefix))
        || TRANSLATE_OFF_COMMANDS.iter().any(|cmd| text == *cmd)
}

fn strip_translate_on_prefix(text: &str) -> Option<&str> {
    let text = text.trim();
    TRANSLATE_ON_PREFIXES
        .iter()
        .find_map(|prefix| text.strip_prefix(prefix))
        .map(str::trim)
}

fn is_bare_translate_on(text: &str) -> bool {
    let text = text.trim();
    TRANSLATE_ON_PREFIXES.iter().any(|prefix| text == *prefix)
}

pub struct TranslateAllHandler {
    store: Arc<GroupPreferencesStore>,
    near_ai: Arc<NearAiClient>,
    signal: Arc<SignalClient>,
}

impl TranslateAllHandler {
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
        is_translate_on_or_off_command(text)
    }

    fn is_text_intercept(message: &BotMessage) -> bool {
        let text = message.text.trim();
        message.group_id.is_some()
            && !message.is_voice_note()
            && !text.is_empty()
            && !text.starts_with('!')
    }

    fn parse_lang_pair(text: &str) -> Option<(&str, &str)> {
        let rest = strip_translate_on_prefix(text)?;
        let mut parts = rest.split_whitespace();
        let a = parts.next()?;
        let b = parts.next()?;
        if parts.next().is_some() {
            return None;
        }
        Some((a, b))
    }

    fn require_group(message: &BotMessage) -> Result<&str, &'static str> {
        message
            .group_id
            .as_deref()
            .ok_or(GROUP_ONLY_MSG)
    }

    async fn handle_setup(&self, message: &BotMessage) -> AppResult<String> {
        let group_id = match Self::require_group(message) {
            Ok(id) => id,
            Err(msg) => return Ok(msg.into()),
        };

        let text = message.text.trim();
        if is_bare_translate_on(text) {
            return Ok(BARE_COMMAND_MSG.into());
        }

        let (token_a, token_b) = match Self::parse_lang_pair(text) {
            Some(pair) => pair,
            None => return Ok(BARE_COMMAND_MSG.into()),
        };

        let lang_a = match resolve_language(token_a) {
            Some(l) => l,
            None => {
                return Ok(format!(
                    "Unknown language: {token_a}. Use !list-langs for supported codes."
                ));
            }
        };
        let lang_b = match resolve_language(token_b) {
            Some(l) => l,
            None => {
                return Ok(format!(
                    "Unknown language: {token_b}. Use !list-langs for supported codes."
                ));
            }
        };

        if lang_a.code == lang_b.code {
            return Ok("Choose two different languages. Example: !translate-on es en".into());
        }

        let mode = GroupTranslateMode::new(lang_a, lang_b);
        let pair_label = mode.display_pair();
        self.store.set(group_id.to_string(), mode);

        info!(group_id, pair = %pair_label, "translate-all mode enabled");
        Ok(format!("Group translate enabled: {pair_label}"))
    }

    async fn handle_off(&self, message: &BotMessage) -> AppResult<String> {
        let group_id = match Self::require_group(message) {
            Ok(id) => id,
            Err(msg) => return Ok(msg.into()),
        };

        if self.store.clear(group_id) {
            info!(group_id, "translate-all mode disabled");
            Ok("Group translate disabled".into())
        } else {
            Ok("Group translate was not active in this chat.".into())
        }
    }

    async fn handle_text_intercept(&self, message: &BotMessage) -> AppResult<()> {
        let group_id = match message.group_id.as_deref() {
            Some(id) => id,
            None => return Ok(()),
        };

        let mode = match self.store.get(group_id) {
            Some(m) => m,
            None => return Ok(()),
        };

        if !self.store.allow_message(group_id) {
            warn!(group_id, "translate-all rate limited — skipping text message");
            return Ok(());
        }

        let (source, target) = match target_for_message_text(&mode, message.text.trim()) {
            Some(pair) => pair,
            None => {
                debug!(
                    group_id,
                    text_chars = message.text.trim().len(),
                    "translate-all skipped text (language not in pair or undetected)"
                );
                return Ok(());
            }
        };

        let translation = match near_ai_translate(&self.near_ai, message.text.trim(), target).await
        {
            Ok(t) => t,
            Err(e) => {
                warn!("translate-all text translation failed: {}", e);
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
            source_lang = source.code,
            target_lang = target.code,
            translation_chars = translation.len(),
            "translate-all text translated"
        );
        Ok(())
    }

    #[instrument(skip(self, message), fields(source = %message.source, is_group = message.is_group))]
    async fn handle_command(&self, message: &BotMessage) -> AppResult<String> {
        let text = message.text.trim();
        if TRANSLATE_OFF_COMMANDS.iter().any(|cmd| text == *cmd) {
            self.handle_off(message).await
        } else {
            self.handle_setup(message).await
        }
    }
}

#[async_trait]
impl CommandHandler for TranslateAllHandler {
    fn label(&self) -> &'static str {
        "translate_all"
    }

    fn matches(&self, message: &BotMessage) -> bool {
        if Self::is_command(&message.text) {
            return true;
        }
        if Self::is_text_intercept(message) {
            if let Some(gid) = &message.group_id {
                return self.store.is_active(gid);
            }
        }
        false
    }

    fn handles_own_reply(&self) -> bool {
        true
    }

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
    use signal_client::BotMessage;

    fn test_handler() -> TranslateAllHandler {
        TranslateAllHandler::new(
            GroupPreferencesStore::new_in_memory(30),
            Arc::new(
                NearAiClient::new("key", "http://localhost", "model", std::time::Duration::from_secs(5))
                    .unwrap(),
            ),
            Arc::new(SignalClient::new("http://localhost").unwrap()),
        )
    }

    #[test]
    fn parse_lang_pair_from_command() {
        assert_eq!(
            TranslateAllHandler::parse_lang_pair("!translate-on es en"),
            Some(("es", "en"))
        );
        assert_eq!(
            TranslateAllHandler::parse_lang_pair("!translation-on es en"),
            Some(("es", "en"))
        );
        assert!(TranslateAllHandler::parse_lang_pair("!translate-on").is_none());
        assert!(TranslateAllHandler::parse_lang_pair("!translation-on").is_none());
        assert!(TranslateAllHandler::parse_lang_pair("!translate-on es en fr").is_none());
    }

    #[test]
    fn is_translate_on_or_off_command_recognizes_aliases() {
        assert!(is_translate_on_or_off_command("!translate-on es en"));
        assert!(is_translate_on_or_off_command("!translation-on"));
        assert!(is_translate_on_or_off_command("!translation-off"));
        assert!(!is_translate_on_or_off_command("!translate es"));
    }

    #[test]
    fn intercept_matches_group_text_when_active() {
        let handler = test_handler();
        let mut msg = BotMessage {
            source: "+1".into(),
            source_number: None,
            source_name: None,
            text: "Hola".into(),
            timestamp: 0,
            message_timestamp: 0,
            is_group: true,
            group_id: Some("gid".into()),
            group_name: None,
            receiving_account: "+2".into(),
            attachments: vec![],
            quote: None,
        };

        assert!(!handler.matches(&msg));
        handler.store.set(
            "gid".into(),
            GroupTranslateMode::new(
                resolve_language("es").unwrap(),
                resolve_language("en").unwrap(),
            ),
        );
        assert!(handler.matches(&msg));

        msg.text = "!help".into();
        assert!(!handler.matches(&msg));
    }
}
