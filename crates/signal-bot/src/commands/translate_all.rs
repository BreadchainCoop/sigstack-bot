//! In-chat auto-translate: `!translate-all-on/off`, `!translate-me-on/off`, `!enable-threads`.

use crate::bot_identity::BotIdentity;
use crate::commands::translate_lang::resolve_language;
use crate::commands::translate_me::TranslateMeHandler;
use crate::commands::translate_service::{
    format_text_auto_translation, near_ai_translate, strip_transcript_prefix,
    target_for_message_text, DEFAULT_TRANSCRIPT_PREFIX,
};
use crate::commands::CommandHandler;
use crate::error::AppResult;
use crate::group_preferences_store::{GroupPreferencesStore, GroupTranslateMode, PendingSwitch};
use async_trait::async_trait;
use near_ai_client::NearAiClient;
use signal_client::{BotMessage, SignalClient};
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

const ALL_ON_PREFIXES: &[&str] = &[
    "!translate-all-on",
    "!translation-all-on",
    "!translate-on",
    "!translation-on",
];
const ALL_OFF_COMMANDS: &[&str] = &[
    "!translate-all-off",
    "!translation-all-off",
    "!translate-off",
    "!translation-off",
];
const ME_ON_PREFIXES: &[&str] = &["!translate-me-on", "!translation-me-on"];
const ME_OFF_COMMANDS: &[&str] = &["!translate-me-off", "!translation-me-off"];
/// Tear down in-chat auto so Language Threads can run (`!enable-threads`).
const ENABLE_THREADS: &[&str] = &["!enable-threads", "!translation-enable-threads"];

const BARE_ALL_MSG: &str = "Please specify two languages. Example: !translate-all-on es en";
const BARE_ME_MSG: &str = "Please specify two languages. Example: !translate-me-on es en";
const GROUP_ONLY_MSG: &str = "In-chat auto-translate is only available in group chats";
const SIDECAR_REJECT_MSG: &str =
    "In-chat auto-translate is only available in the main group (not a Language Thread).";
const THREADS_BLOCK_MSG: &str = "Language Threads is already on in this group, so in-chat auto-translate can't run alongside it.\n\nTo switch, send:\n!enable-in-chat";

/// Whether the message is any in-chat auto on/off/disable command (excludes quote `!translate`).
pub(crate) fn is_translate_on_or_off_command(text: &str) -> bool {
    let text = text.trim();
    is_all_on_command(text)
        || ALL_OFF_COMMANDS.contains(&text)
        || is_me_on_command(text)
        || ME_OFF_COMMANDS.contains(&text)
        || ENABLE_THREADS.contains(&text)
}

fn starts_with_word(text: &str, prefix: &str) -> bool {
    text == prefix
        || text
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(' '))
}

fn is_all_on_command(text: &str) -> bool {
    ALL_ON_PREFIXES.iter().any(|p| starts_with_word(text, p))
}

fn is_me_on_command(text: &str) -> bool {
    ME_ON_PREFIXES.iter().any(|p| starts_with_word(text, p))
}

fn strip_prefix_list<'a>(text: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    prefixes.iter().find_map(|prefix| {
        if text == *prefix {
            Some("")
        } else {
            text.strip_prefix(prefix)
                .filter(|rest| rest.is_empty() || rest.starts_with(' '))
                .map(str::trim)
        }
    })
}

fn is_bare_all_on(text: &str) -> bool {
    ALL_ON_PREFIXES.contains(&text.trim())
}

fn is_bare_me_on(text: &str) -> bool {
    ME_ON_PREFIXES.contains(&text.trim())
}

#[derive(Clone)]
pub struct TranslateAllHandler {
    store: Arc<GroupPreferencesStore>,
    near_ai: Arc<NearAiClient>,
    signal: Arc<SignalClient>,
    bot_identity: Option<Arc<BotIdentity>>,
    transcript_prefix: String,
}

impl TranslateAllHandler {
    pub fn new(
        store: Arc<GroupPreferencesStore>,
        near_ai: Arc<NearAiClient>,
        signal: Arc<SignalClient>,
    ) -> Self {
        Self::with_prefix(store, near_ai, signal, DEFAULT_TRANSCRIPT_PREFIX)
    }

    pub fn with_prefix(
        store: Arc<GroupPreferencesStore>,
        near_ai: Arc<NearAiClient>,
        signal: Arc<SignalClient>,
        transcript_prefix: impl Into<String>,
    ) -> Self {
        Self {
            store,
            near_ai,
            signal,
            bot_identity: None,
            transcript_prefix: transcript_prefix.into(),
        }
    }

    pub fn with_bot_identity(mut self, bot_identity: Arc<BotIdentity>) -> Self {
        self.bot_identity = Some(bot_identity);
        self
    }

    /// In-chat translate a transcript as if the original speaker posted it.
    pub(crate) async fn fan_out_transcript(&self, original: &BotMessage, spoken: &str) {
        if spoken.trim().is_empty() {
            return;
        }
        let mut msg = original.clone();
        msg.text = format!("{}\n{spoken}", self.transcript_prefix);
        if let Err(e) = self.handle_text_intercept(&msg).await {
            warn!(error = %e, "in-chat fan-out after transcript failed");
        }
    }

    fn is_command(text: &str) -> bool {
        is_translate_on_or_off_command(text)
    }

    /// Group text eligible for auto-translate (commands excluded).
    ///
    /// Allows text even when audio attachments are present so transcription-bot
    /// quote-replies that still carry voice metadata are not skipped.
    fn is_text_intercept(message: &BotMessage) -> bool {
        let text = message.text.trim();
        message.group_id.is_some() && !text.is_empty() && !text.starts_with('!')
    }

    fn looks_like_transcript(&self, text: &str) -> bool {
        let prefix = if self.transcript_prefix.is_empty() {
            DEFAULT_TRANSCRIPT_PREFIX
        } else {
            self.transcript_prefix.as_str()
        };
        text.trim().starts_with(prefix)
    }

    /// Spoken body for detect/translate (strip Whisper label when present).
    fn intercept_text(&self, message: &BotMessage) -> String {
        strip_transcript_prefix(message.text.trim(), &self.transcript_prefix)
    }

    /// Resolve in-chat mode; for peer/transcript posts, try quote author first.
    fn resolve_mode_for_message(
        &self,
        group_id: &str,
        message: &BotMessage,
    ) -> Option<GroupTranslateMode> {
        let treat_as_transcript = self.looks_like_transcript(&message.text);
        if treat_as_transcript {
            if let Some(author) = message
                .quote
                .as_ref()
                .and_then(|q| q.author_number.as_deref())
            {
                if let Some(mode) = self.store.resolve_in_chat_mode(group_id, author) {
                    return Some(mode);
                }
            }
        }

        if let Some(mode) = self.store.resolve_in_chat_mode(group_id, &message.source) {
            return Some(mode);
        }
        if let Some(n) = message.source_number.as_deref() {
            if n != message.source.as_str() {
                return self.store.resolve_in_chat_mode(group_id, n);
            }
        }
        None
    }

    fn set_member_prefs(&self, group_id: &str, message: &BotMessage, mode: GroupTranslateMode) {
        self.store
            .set_member_translate(group_id, &message.source, mode.clone());
        if let Some(n) = message.source_number.as_deref() {
            if n != message.source.as_str() {
                self.store.set_member_translate(group_id, n, mode);
            }
        }
    }

    fn clear_member_prefs(&self, group_id: &str, message: &BotMessage) -> bool {
        let mut cleared = self.store.clear_member_translate(group_id, &message.source);
        if let Some(n) = message.source_number.as_deref() {
            if n != message.source.as_str() {
                cleared = self.store.clear_member_translate(group_id, n) || cleared;
            }
        }
        cleared
    }

    fn parse_lang_pair<'a>(text: &'a str, prefixes: &[&str]) -> Option<(&'a str, &'a str)> {
        let rest = strip_prefix_list(text.trim(), prefixes)?;
        let mut parts = rest.split_whitespace();
        let a = parts.next()?;
        let b = parts.next()?;
        if parts.next().is_some() {
            return None;
        }
        Some((a, b))
    }

    fn require_group(message: &BotMessage) -> Result<&str, &'static str> {
        message.group_id.as_deref().ok_or(GROUP_ONLY_MSG)
    }

    fn resolve_pair_tokens(
        token_a: &str,
        token_b: &str,
        example: &str,
    ) -> Result<GroupTranslateMode, String> {
        let lang_a = resolve_language(token_a).ok_or_else(|| {
            format!("Unknown language: {token_a}. Use !list-langs for supported codes.")
        })?;
        let lang_b = resolve_language(token_b).ok_or_else(|| {
            format!("Unknown language: {token_b}. Use !list-langs for supported codes.")
        })?;
        if lang_a.code == lang_b.code {
            return Err(format!(
                "Choose two different languages. Example: {example}"
            ));
        }
        Ok(GroupTranslateMode::new(lang_a, lang_b))
    }

    async fn refuse_if_threads(&self, group_id: &str, pending: PendingSwitch) -> Option<String> {
        if !self.store.threads_active(group_id) {
            return None;
        }
        self.store.set_pending_switch(group_id, pending);
        Some(THREADS_BLOCK_MSG.into())
    }

    async fn handle_all_on(&self, message: &BotMessage) -> AppResult<String> {
        let group_id = match Self::require_group(message) {
            Ok(id) => id,
            Err(msg) => return Ok(msg.into()),
        };
        if self.store.lookup_sidecar(group_id).is_some() {
            return Ok(SIDECAR_REJECT_MSG.into());
        }

        let text = message.text.trim();
        if is_bare_all_on(text) {
            return Ok(BARE_ALL_MSG.into());
        }
        let Some((token_a, token_b)) = Self::parse_lang_pair(text, ALL_ON_PREFIXES) else {
            return Ok(BARE_ALL_MSG.into());
        };
        let mode = match Self::resolve_pair_tokens(token_a, token_b, "!translate-all-on es en") {
            Ok(m) => m,
            Err(e) => return Ok(e),
        };

        if let Some(msg) = self
            .refuse_if_threads(
                group_id,
                PendingSwitch::EnableAllOn {
                    user: message.source.clone(),
                    lang_a: mode.lang_a.clone(),
                    lang_b: mode.lang_b.clone(),
                },
            )
            .await
        {
            return Ok(msg);
        }

        let pair_label = mode.display_pair();
        self.store.set(group_id.to_string(), mode);
        info!(group_id, pair = %pair_label, "translate-all mode enabled");
        Ok(format!("Group translate enabled: {pair_label}"))
    }

    async fn handle_me_on(&self, message: &BotMessage) -> AppResult<String> {
        let group_id = match Self::require_group(message) {
            Ok(id) => id,
            Err(msg) => return Ok(msg.into()),
        };
        if self.store.lookup_sidecar(group_id).is_some() {
            return Ok(SIDECAR_REJECT_MSG.into());
        }

        let text = message.text.trim();
        if is_bare_me_on(text) {
            return Ok(BARE_ME_MSG.into());
        }
        let Some((token_a, token_b)) = Self::parse_lang_pair(text, ME_ON_PREFIXES) else {
            return Ok(BARE_ME_MSG.into());
        };
        let mode = match Self::resolve_pair_tokens(token_a, token_b, "!translate-me-on es en") {
            Ok(m) => m,
            Err(e) => return Ok(e),
        };

        if let Some(msg) = self
            .refuse_if_threads(
                group_id,
                PendingSwitch::EnableMeOn {
                    user: message.source.clone(),
                    lang_a: mode.lang_a.clone(),
                    lang_b: mode.lang_b.clone(),
                },
            )
            .await
        {
            return Ok(msg);
        }

        let pair_label = mode.display_pair();
        self.set_member_prefs(group_id, message, mode);
        info!(
            group_id,
            user = %message.source,
            pair = %pair_label,
            "translate-me (in-chat) enabled"
        );
        Ok(format!("Personal translate enabled: {pair_label}"))
    }

    async fn handle_all_off(&self, message: &BotMessage) -> AppResult<String> {
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

    async fn handle_me_off(&self, message: &BotMessage) -> AppResult<String> {
        let group_id = match Self::require_group(message) {
            Ok(id) => id,
            Err(msg) => return Ok(msg.into()),
        };
        if self.clear_member_prefs(group_id, message) {
            info!(group_id, user = %message.source, "translate-me (in-chat) disabled");
            Ok("Personal translate disabled".into())
        } else {
            Ok("Personal translate was not active for you in this chat.".into())
        }
    }

    async fn handle_enable_threads(&self, message: &BotMessage) -> AppResult<String> {
        let group_id = match Self::require_group(message) {
            Ok(id) => id,
            Err(msg) => return Ok(msg.into()),
        };

        let (had, pending) = self.store.disable_in_chat_and_take_pending(group_id);

        let mut parts = Vec::new();
        if had {
            parts.push("In-chat auto-translate disabled.".to_string());
        } else {
            parts.push("In-chat auto-translate was not active in this chat.".to_string());
        }

        if let Some(PendingSwitch::EnableThreads {
            user,
            lang,
            address,
        }) = pending
        {
            let applied = TranslateMeHandler::subscribe_user_to_thread(
                &self.store,
                &self.signal,
                message,
                group_id,
                &lang,
                Some(user.as_str()),
                address.as_deref(),
            )
            .await?;
            parts.push(applied);
        } else if let Some(other) = pending {
            // Restore unexpected pending rather than drop silently.
            self.store.set_pending_switch(group_id, other);
            parts.push(
                "Pending switch was not a Language Threads subscribe; left unchanged. \
                 Use !translation-threads for Language Threads."
                    .into(),
            );
        } else if had {
            parts.push("You can enable Language Threads with !translate-me-thread <lang>.".into());
        }

        Ok(parts.join(" "))
    }

    async fn handle_text_intercept(&self, message: &BotMessage) -> AppResult<()> {
        let group_id = match message.group_id.as_deref() {
            Some(id) => id,
            None => return Ok(()),
        };

        // Threads still wins via handler order; skip if somehow both configured.
        if self.store.threads_active(group_id) {
            return Ok(());
        }

        let mode = match self.resolve_mode_for_message(group_id, message) {
            Some(m) => m,
            None => return Ok(()),
        };

        if !self.store.allow_message(group_id) {
            warn!(
                group_id,
                "translate-all rate limited — skipping text message"
            );
            return Ok(());
        }

        let spoken = self.intercept_text(message);
        if spoken.is_empty() {
            return Ok(());
        }

        let (source, target) = match target_for_message_text(&mode, &spoken) {
            Some(pair) => pair,
            None => {
                debug!(
                    group_id,
                    text_chars = spoken.len(),
                    "translate-all skipped text (language not in pair or undetected)"
                );
                return Ok(());
            }
        };

        let translation = match near_ai_translate(&self.near_ai, &spoken, target).await {
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
        if ENABLE_THREADS.contains(&text) {
            self.handle_enable_threads(message).await
        } else if ME_OFF_COMMANDS.contains(&text) {
            self.handle_me_off(message).await
        } else if ALL_OFF_COMMANDS.contains(&text) {
            self.handle_all_off(message).await
        } else if is_me_on_command(text) {
            self.handle_me_on(message).await
        } else {
            self.handle_all_on(message).await
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
        if self
            .bot_identity
            .as_ref()
            .is_some_and(|id| id.is_bot_message(message))
        {
            return false;
        }
        if Self::is_text_intercept(message) {
            if let Some(gid) = &message.group_id {
                return self.resolve_mode_for_message(gid, message).is_some()
                    && !self.store.threads_active(gid);
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
    use crate::commands::translate_lang::resolve_language;
    use crate::group_preferences_store::{GroupTranslateMode, PendingSwitch};
    use signal_client::BotMessage;

    fn test_handler() -> TranslateAllHandler {
        TranslateAllHandler::new(
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

    #[test]
    fn parse_lang_pair_from_command() {
        assert_eq!(
            TranslateAllHandler::parse_lang_pair("!translate-all-on es en", ALL_ON_PREFIXES),
            Some(("es", "en"))
        );
        assert_eq!(
            TranslateAllHandler::parse_lang_pair("!translate-on es en", ALL_ON_PREFIXES),
            Some(("es", "en"))
        );
        assert_eq!(
            TranslateAllHandler::parse_lang_pair("!translate-me-on es en", ME_ON_PREFIXES),
            Some(("es", "en"))
        );
        assert!(
            TranslateAllHandler::parse_lang_pair("!translate-all-on", ALL_ON_PREFIXES).is_none()
        );
        assert!(TranslateAllHandler::parse_lang_pair(
            "!translate-all-on es en fr",
            ALL_ON_PREFIXES
        )
        .is_none());
    }

    #[test]
    fn is_translate_on_or_off_command_recognizes_aliases() {
        assert!(is_translate_on_or_off_command("!translate-all-on es en"));
        assert!(is_translate_on_or_off_command("!translate-on es en"));
        assert!(is_translate_on_or_off_command("!translate-me-on es en"));
        assert!(is_translate_on_or_off_command("!translate-me-off"));
        assert!(is_translate_on_or_off_command("!enable-threads"));
        assert!(is_translate_on_or_off_command("!translation-off"));
        assert!(!is_translate_on_or_off_command("!translate es"));
        assert!(!is_translate_on_or_off_command("!translate-me-thread es"));
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

    #[test]
    fn personal_intercept_only_for_subscriber() {
        let handler = test_handler();
        let mode = GroupTranslateMode::new(
            resolve_language("es").unwrap(),
            resolve_language("en").unwrap(),
        );
        handler.store.set_member_translate("gid", "+alice", mode);

        let alice = BotMessage {
            source: "+alice".into(),
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
        let bob = BotMessage {
            source: "+bob".into(),
            ..alice.clone()
        };
        assert!(handler.matches(&alice));
        assert!(!handler.matches(&bob));
    }

    #[test]
    fn group_wide_matches_transcript_even_with_audio_attachment() {
        use signal_client::Attachment;

        let handler = test_handler();
        handler.store.set(
            "gid".into(),
            GroupTranslateMode::new(
                resolve_language("es").unwrap(),
                resolve_language("en").unwrap(),
            ),
        );

        let msg = BotMessage {
            source: "+15550009999".into(),
            source_number: Some("+15550009999".into()),
            source_name: Some("Transcription".into()),
            text: "📝 Transcript:\nHola, ¿cómo estás?".into(),
            timestamp: 0,
            message_timestamp: 0,
            is_group: true,
            group_id: Some("gid".into()),
            group_name: None,
            receiving_account: "+15550001111".into(),
            attachments: vec![Attachment {
                content_type: "audio/aac".into(),
                filename: Some("voice.m4a".into()),
                id: "att-1".into(),
                size: Some(100),
                upload_timestamp: None,
            }],
            quote: None,
        };
        assert!(msg.is_voice_note());
        assert!(handler.matches(&msg));
    }

    #[test]
    fn personal_matches_transcript_via_quote_author() {
        use signal_client::QuotedMessage;

        let mode = GroupTranslateMode::new(
            resolve_language("es").unwrap(),
            resolve_language("en").unwrap(),
        );
        let handler = TranslateAllHandler::with_prefix(
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
            DEFAULT_TRANSCRIPT_PREFIX,
        );
        handler.store.set_member_translate("gid", "+alice", mode);

        let msg = BotMessage {
            source: "+15550001111".into(),
            source_number: Some("+15550001111".into()),
            source_name: Some("Bread Bot".into()),
            text: "📝 Transcript:\nHola amigos".into(),
            timestamp: 0,
            message_timestamp: 0,
            is_group: true,
            group_id: Some("gid".into()),
            group_name: None,
            receiving_account: "+15550001111".into(),
            attachments: vec![],
            quote: Some(QuotedMessage {
                id: 1,
                author_number: Some("+alice".into()),
                text: None,
                audio_attachment: None,
            }),
        };
        assert!(handler.matches(&msg));

        let other = BotMessage {
            quote: Some(QuotedMessage {
                id: 2,
                author_number: Some("+bob".into()),
                text: None,
                audio_attachment: None,
            }),
            ..msg.clone()
        };
        assert!(!handler.matches(&other));
    }

    #[test]
    fn fan_out_message_uses_speaker_source() {
        let mode = GroupTranslateMode::new(
            resolve_language("es").unwrap(),
            resolve_language("en").unwrap(),
        );
        let handler = TranslateAllHandler::with_prefix(
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
            DEFAULT_TRANSCRIPT_PREFIX,
        );
        handler.store.set_member_translate("gid", "+alice", mode);

        let msg = BotMessage {
            source: "+alice".into(),
            source_number: Some("+alice".into()),
            source_name: Some("Alice".into()),
            text: format!("{}\nHola amigos", DEFAULT_TRANSCRIPT_PREFIX),
            timestamp: 0,
            message_timestamp: 0,
            is_group: true,
            group_id: Some("gid".into()),
            group_name: None,
            receiving_account: "+15550001111".into(),
            attachments: vec![],
            quote: None,
        };
        assert!(handler.resolve_mode_for_message("gid", &msg).is_some());
        assert!(handler.matches(&msg));
    }

    #[test]
    fn bot_translation_replies_do_not_match() {
        let mode = GroupTranslateMode::new(
            resolve_language("es").unwrap(),
            resolve_language("en").unwrap(),
        );
        let identity = BotIdentity::new();
        identity.remember_phone("+15550001111");
        let handler = TranslateAllHandler::with_prefix(
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
            DEFAULT_TRANSCRIPT_PREFIX,
        )
        .with_bot_identity(identity);
        handler.store.set("gid".into(), mode);

        let msg = BotMessage {
            source: "+15550001111".into(),
            source_number: Some("+15550001111".into()),
            source_name: Some("Bread Bot".into()),
            text: "Hello friends".into(),
            timestamp: 0,
            message_timestamp: 0,
            is_group: true,
            group_id: Some("gid".into()),
            group_name: None,
            receiving_account: "+15550001111".into(),
            attachments: vec![],
            quote: None,
        };
        assert!(!handler.matches(&msg));
    }

    #[tokio::test]
    async fn fan_out_transcript_skips_empty_spoken() {
        let handler = TranslateAllHandler::with_prefix(
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
            DEFAULT_TRANSCRIPT_PREFIX,
        );
        let msg = BotMessage {
            source: "+alice".into(),
            source_number: Some("+alice".into()),
            source_name: None,
            text: String::new(),
            timestamp: 0,
            message_timestamp: 0,
            is_group: true,
            group_id: Some("gid".into()),
            group_name: None,
            receiving_account: "+bot".into(),
            attachments: vec![],
            quote: None,
        };
        handler.fan_out_transcript(&msg, "  ").await;
    }

    #[tokio::test]
    async fn execute_setup_commands_send_replies() {
        use serde_json::json;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let signal = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(7)
            .mount(&signal)
            .await;

        let store = GroupPreferencesStore::new_in_memory(30);
        let handler = TranslateAllHandler::new(
            store.clone(),
            Arc::new(
                NearAiClient::new(
                    "key",
                    "http://127.0.0.1:9",
                    "model",
                    std::time::Duration::from_secs(2),
                )
                .unwrap(),
            ),
            Arc::new(SignalClient::new(signal.uri()).unwrap()),
        );

        let mut msg = BotMessage {
            source: "+15550002222".into(),
            source_number: Some("+15550002222".into()),
            source_name: None,
            text: "!translate-all-on".into(),
            timestamp: 1,
            message_timestamp: 1,
            is_group: false,
            group_id: None,
            group_name: None,
            receiving_account: "+15550001111".into(),
            attachments: vec![],
            quote: None,
        };

        assert!(handler.execute(&msg).await.unwrap().is_empty());

        msg.is_group = true;
        msg.group_id = Some("group.main".into());
        assert!(handler.execute(&msg).await.unwrap().is_empty());

        msg.text = "!translate-all-on xx yy".into();
        assert!(handler.execute(&msg).await.unwrap().is_empty());

        msg.text = "!translate-all-on es en".into();
        assert!(handler.execute(&msg).await.unwrap().is_empty());
        assert!(store.is_active("group.main"));

        msg.text = "!translate-me-on fr en".into();
        assert!(handler.execute(&msg).await.unwrap().is_empty());
        assert!(store
            .get_member_translate("group.main", "+15550002222")
            .is_some());

        msg.text = "!translate-all-off".into();
        assert!(handler.execute(&msg).await.unwrap().is_empty());
        assert!(!store.is_active("group.main"));
        assert!(store.in_chat_auto_active("group.main"));

        msg.text = "!translate-me-off".into();
        assert!(handler.execute(&msg).await.unwrap().is_empty());
        assert!(!store.in_chat_auto_active("group.main"));
    }

    #[tokio::test]
    async fn refuses_when_threads_active_and_disable_applies_pending() {
        use serde_json::json;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let signal = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&signal)
            .await;

        let store = GroupPreferencesStore::new_in_memory(30);
        store.set_sidecar("group.main", "es", "group.es".into(), "es-internal".into());
        let handler = TranslateAllHandler::new(
            store.clone(),
            Arc::new(
                NearAiClient::new(
                    "key",
                    "http://127.0.0.1:9",
                    "model",
                    std::time::Duration::from_secs(2),
                )
                .unwrap(),
            ),
            Arc::new(SignalClient::new(signal.uri()).unwrap()),
        );

        let msg = BotMessage {
            source: "+15550002222".into(),
            source_number: Some("+15550002222".into()),
            source_name: None,
            text: "!translate-all-on es en".into(),
            timestamp: 1,
            message_timestamp: 1,
            is_group: true,
            group_id: Some("group.main".into()),
            group_name: None,
            receiving_account: "+15550001111".into(),
            attachments: vec![],
            quote: None,
        };
        assert!(handler.execute(&msg).await.unwrap().is_empty());
        assert!(!store.is_active("group.main"));
        assert!(matches!(
            store.get_pending_switch("group.main"),
            Some(PendingSwitch::EnableAllOn { .. })
        ));
    }

    #[tokio::test]
    async fn enable_threads_applies_pending_subscribe() {
        use serde_json::json;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let signal = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&signal)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/groups/%2B15550001111"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "group.es"})))
            .mount(&signal)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/groups/%2B15550001111"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {
                    "name": "Language Thread Spanish",
                    "id": "group.es",
                    "internal_id": "es-internal"
                }
            ])))
            .mount(&signal)
            .await;

        let store = GroupPreferencesStore::new_in_memory(0);
        let mode = GroupTranslateMode::new(
            resolve_language("es").unwrap(),
            resolve_language("en").unwrap(),
        );
        store.set_member_translate("group.main", "+15550002222", mode);
        store.set_pending_switch(
            "group.main",
            PendingSwitch::EnableThreads {
                user: "+15550002222".into(),
                lang: "es".into(),
                address: Some("+15550002222".into()),
            },
        );

        let handler = TranslateAllHandler::new(
            store.clone(),
            Arc::new(
                NearAiClient::new(
                    "key",
                    "http://127.0.0.1:9",
                    "model",
                    std::time::Duration::from_secs(2),
                )
                .unwrap(),
            ),
            Arc::new(SignalClient::new(signal.uri()).unwrap()),
        );

        let msg = BotMessage {
            source: "+15550002222".into(),
            source_number: Some("+15550002222".into()),
            source_name: None,
            text: "!enable-threads".into(),
            timestamp: 1,
            message_timestamp: 1,
            is_group: true,
            group_id: Some("group.main".into()),
            group_name: None,
            receiving_account: "+15550001111".into(),
            attachments: vec![],
            quote: None,
        };
        assert!(handler.execute(&msg).await.unwrap().is_empty());
        assert!(store.threads_active("group.main"));
        assert_eq!(
            store.member_lang("group.main", "+15550002222"),
            Some("es".into())
        );
        assert_eq!(
            store.lookup_sidecar("es-internal"),
            Some(("group.main".into(), "es".into()))
        );
        assert!(!store.in_chat_auto_active("group.main"));
    }

    #[tokio::test]
    async fn execute_intercept_translates_group_text() {
        use serde_json::json;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let signal = MockServer::start().await;
        let near = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&signal)
            .await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "1",
                "choices": [{"index": 0, "message": {"role": "assistant", "content": "Hello"}, "finish_reason": "stop"}],
                "created": 1,
                "model": "m",
                "object": "chat.completion"
            })))
            .mount(&near)
            .await;

        let store = GroupPreferencesStore::new_in_memory(30);
        store.set(
            "group.main".into(),
            GroupTranslateMode::new(
                resolve_language("es").unwrap(),
                resolve_language("en").unwrap(),
            ),
        );
        let handler = TranslateAllHandler::new(
            store,
            Arc::new(
                NearAiClient::new("key", near.uri(), "m", std::time::Duration::from_secs(5))
                    .unwrap(),
            ),
            Arc::new(SignalClient::new(signal.uri()).unwrap()),
        );

        let msg = BotMessage {
            source: "+15550002222".into(),
            source_number: Some("+15550002222".into()),
            source_name: None,
            text: "Hola amigos".into(),
            timestamp: 1,
            message_timestamp: 1,
            is_group: true,
            group_id: Some("group.main".into()),
            group_name: None,
            receiving_account: "+15550001111".into(),
            attachments: vec![],
            quote: None,
        };
        assert!(handler.matches(&msg));
        assert!(handler.execute(&msg).await.unwrap().is_empty());
    }
}
