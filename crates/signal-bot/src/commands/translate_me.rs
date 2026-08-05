//! Language Threads: `!translate-me-thread` / `!leave` / `!enable-in-chat` + relay engine.
//!
//! Main group stays multilingual. Each subscribed language gets a
//! `{Language} · {disambiguator}` Signal sidecar. Messages fan out:
//! main→sidecars (relay/translate), sidecar→main (relay) + other sidecars (translate).
//! Bot never relays itself.

use crate::bot_identity::BotIdentity;
use crate::commands::translate_lang::{resolve_language, Language};
use crate::commands::translate_service::{
    detect_text_language, near_ai_translate, strip_transcript_prefix, DEFAULT_TRANSCRIPT_PREFIX,
};
use crate::commands::CommandHandler;
use crate::error::AppResult;
use crate::group_preferences_store::{
    GroupPreferencesStore, GroupTranslateMode, LanguageBridge, PendingSwitch,
};
use async_trait::async_trait;
use near_ai_client::NearAiClient;
use signal_client::{BotMessage, SignalClient};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

const GROUP_ONLY_MSG: &str =
    "!translate-me-thread is only available in the main mutual-aid group (not DMs).";
const SIDECAR_ON_MSG: &str =
    "Subscribe from the main group with !translate-me-thread <lang>. Use !leave here to leave.";
const USAGE_MSG: &str = "Usage: !translate-me-thread <lang> (e.g. !translate-me-thread es)";
const NO_ADDRESS_MSG: &str = "Could not invite you: Signal did not include your phone number. \
Message this bot in a 1:1 chat once, then retry !translate-me-thread <lang>.";
const LEAVE_SIDECAR_ONLY_MSG: &str =
    "!leave is only available inside a Language Thread. Open that chat and send !leave (or !commands).";
const IN_CHAT_BLOCK_MSG: &str = "In-chat auto-translate is already on in this group, so Language Threads can't start alongside it.\n\nTo switch, send:\n!enable-threads";
/// Tear down Language Threads so in-chat can run (`!enable-in-chat`).
const ENABLE_IN_CHAT_CMDS: &[&str] = &["!enable-in-chat", "!translation-enable-in-chat"];
const THREADS_DISABLED_SIDECAR_MSG: &str = "Language Threads were disabled in the main group (in-chat translation is on).\n\nReturn to the main chat to continue — this thread will no longer relay messages.";
const LEAVE_CMDS: &[&str] = &["!leave"];

pub struct TranslateMeHandler {
    store: Arc<GroupPreferencesStore>,
    near_ai: Arc<NearAiClient>,
    signal: Arc<SignalClient>,
    bot_identity: Arc<BotIdentity>,
}

impl TranslateMeHandler {
    pub fn new(
        store: Arc<GroupPreferencesStore>,
        near_ai: Arc<NearAiClient>,
        signal: Arc<SignalClient>,
        bot_identity: Arc<BotIdentity>,
    ) -> Self {
        Self {
            store,
            near_ai,
            signal,
            bot_identity,
        }
    }

    fn is_on_command(text: &str) -> bool {
        let t = text.trim();
        starts_with_word(t, "!translate-me-thread") || starts_with_word(t, "!translation-me-thread")
    }

    fn is_off_command(text: &str) -> bool {
        LEAVE_CMDS.contains(&text.trim())
    }

    fn is_enable_in_chat(text: &str) -> bool {
        ENABLE_IN_CHAT_CMDS.contains(&text.trim())
    }

    fn is_command(text: &str) -> bool {
        let t = text.trim();
        Self::is_on_command(t) || Self::is_off_command(t) || Self::is_enable_in_chat(t)
    }

    fn on_lang_arg(text: &str) -> Option<&str> {
        let t = text.trim();
        for prefix in ["!translate-me-thread", "!translation-me-thread"] {
            if let Some(rest) = strip_word_prefix(t, prefix) {
                return rest.split_whitespace().next();
            }
        }
        None
    }

    fn is_relay_candidate(&self, message: &BotMessage) -> bool {
        let text = message.text.trim();
        if message.group_id.is_none() || text.is_empty() || text.starts_with('!') {
            return false;
        }
        let Some(gid) = message.group_id.as_deref() else {
            return false;
        };
        self.store.get_bridge(gid).is_some() || self.store.lookup_sidecar(gid).is_some()
    }

    async fn handle_command(&self, message: &BotMessage) -> AppResult<String> {
        let text = message.text.trim();

        if Self::is_enable_in_chat(text) {
            return self.handle_enable_in_chat(message).await;
        }

        if Self::is_off_command(text) {
            return self.handle_leave(message).await;
        }

        if Self::is_on_command(text) {
            let Some(gid) = message.group_id.as_deref() else {
                return Ok(GROUP_ONLY_MSG.into());
            };

            if self.store.lookup_sidecar(gid).is_some() {
                return Ok(SIDECAR_ON_MSG.into());
            }

            let Some(lang_token) = Self::on_lang_arg(text) else {
                return Ok(USAGE_MSG.into());
            };

            if self.store.in_chat_auto_active(gid) {
                self.store.set_pending_switch(
                    gid,
                    PendingSwitch::EnableThreads {
                        user: message.source.clone(),
                        lang: lang_token.to_string(),
                        address: message.invite_address(),
                    },
                );
                return Ok(IN_CHAT_BLOCK_MSG.into());
            }

            return Self::subscribe_user_to_thread(
                &self.store,
                &self.signal,
                message,
                gid,
                lang_token,
                None,
                None,
            )
            .await;
        }

        Ok(USAGE_MSG.into())
    }

    async fn handle_leave(&self, message: &BotMessage) -> AppResult<String> {
        let Some(gid) = message.group_id.as_deref() else {
            return Ok(LEAVE_SIDECAR_ONLY_MSG.into());
        };

        let Some((main_id, _)) = self.store.lookup_sidecar(gid) else {
            return Ok(LEAVE_SIDECAR_ONLY_MSG.into());
        };

        let user_key = message.source.as_str();
        let Some((lang, stored_addr)) = self.store.clear_bridge_member(&main_id, user_key) else {
            return Ok("You are not subscribed to a language sidecar.".into());
        };

        let address = stored_addr
            .or_else(|| message.invite_address())
            .unwrap_or_else(|| message.source.clone());

        if let Some(bridge) = self.store.get_bridge(&main_id) {
            if let Some(send_id) = bridge.sidecar_send_id(&lang) {
                if let Err(e) = self
                    .signal
                    .remove_members(&message.receiving_account, send_id, vec![address])
                    .await
                {
                    warn!(error = %e, "Failed to remove member from sidecar on leave");
                }
            }
        }

        let lang_name = resolve_language(&lang)
            .map(|l| l.name)
            .unwrap_or(lang.as_str());
        Ok(format!("Left the {lang_name} sidecar."))
    }

    async fn handle_enable_in_chat(&self, message: &BotMessage) -> AppResult<String> {
        let Some(gid) = message.group_id.as_deref() else {
            return Ok("!enable-in-chat is only available in the main group.".into());
        };
        if self.store.lookup_sidecar(gid).is_some() {
            return Ok(
                "!enable-in-chat works from the main group. Use !leave to leave this thread."
                    .into(),
            );
        }

        let Some(bridge) = self.store.take_bridge(gid) else {
            let pending = self.store.take_pending_switch(gid);
            if pending.is_none() {
                return Ok(
                    "Language Threads was not active in this chat. See !translation-threads."
                        .into(),
                );
            }
            return Ok(self
                .apply_pending_in_chat(gid, pending, "Language Threads was not active.")
                .await);
        };

        let bot = message.receiving_account.as_str();
        self.notify_sidecars_threads_disabled(bot, &bridge).await;

        for (user, lang) in &bridge.members {
            let address = bridge
                .member_addresses
                .get(user)
                .cloned()
                .unwrap_or_else(|| user.clone());
            if let Some(send_id) = bridge.sidecar_send_id(lang) {
                if let Err(e) = self
                    .signal
                    .remove_members(bot, send_id, vec![address])
                    .await
                {
                    warn!(error = %e, user = %user, "Failed to remove member during enable-in-chat");
                }
            }
        }

        let pending = self.store.take_pending_switch(gid);
        Ok(self
            .apply_pending_in_chat(gid, pending, "Language Threads disabled.")
            .await)
    }

    async fn notify_sidecars_threads_disabled(&self, bot: &str, bridge: &LanguageBridge) {
        let mut notified = std::collections::HashSet::new();
        for send_id in bridge.sidecars.values() {
            if !notified.insert(send_id.as_str()) {
                continue;
            }
            if let Err(e) = self
                .signal
                .send(bot, send_id, THREADS_DISABLED_SIDECAR_MSG)
                .await
            {
                warn!(
                    error = %e,
                    send_id,
                    "Failed to notify sidecar that Language Threads were disabled"
                );
            } else {
                info!(
                    send_id,
                    "Notified sidecar that Language Threads were disabled"
                );
            }
        }
    }

    async fn apply_pending_in_chat(
        &self,
        group_id: &str,
        pending: Option<PendingSwitch>,
        disabled_prefix: &str,
    ) -> String {
        match pending {
            Some(PendingSwitch::EnableAllOn { lang_a, lang_b, .. }) => {
                if let (Some(a), Some(b)) = (resolve_language(&lang_a), resolve_language(&lang_b)) {
                    let mode = GroupTranslateMode::new(a, b);
                    let pair = mode.display_pair();
                    self.store.set(group_id.to_string(), mode);
                    format!("{disabled_prefix} Group translate enabled: {pair}")
                } else {
                    format!(
                        "{disabled_prefix} Could not apply pending group translate (unknown language)."
                    )
                }
            }
            Some(PendingSwitch::EnableMeOn {
                user,
                lang_a,
                lang_b,
            }) => {
                if let (Some(a), Some(b)) = (resolve_language(&lang_a), resolve_language(&lang_b)) {
                    let mode = GroupTranslateMode::new(a, b);
                    let pair = mode.display_pair();
                    self.store.set_member_translate(group_id, &user, mode);
                    format!("{disabled_prefix} Personal translate enabled: {pair}")
                } else {
                    format!(
                        "{disabled_prefix} Could not apply pending personal translate (unknown language)."
                    )
                }
            }
            Some(other) => {
                self.store.set_pending_switch(group_id, other);
                format!(
                    "{disabled_prefix} You can enable in-chat with !translate-all-on or !translate-me-on."
                )
            }
            None => format!(
                "{disabled_prefix} You can enable in-chat with !translate-all-on or !translate-me-on."
            ),
        }
    }

    #[instrument(skip(self, message))]
    async fn resolve_sidecar_route(
        &self,
        message: &BotMessage,
    ) -> AppResult<Option<(String, String)>> {
        let gid = match message.group_id.as_deref() {
            Some(id) => id,
            None => return Ok(None),
        };

        if let Some(route) = self.store.lookup_sidecar(gid) {
            return Ok(Some(route));
        }

        if gid.starts_with("group.") {
            if let Some(route) = self.store.lookup_sidecar_by_send_id(gid) {
                return Ok(Some(route));
            }
        }

        match self.signal.list_groups(&message.receiving_account).await {
            Ok(groups) => Ok(self
                .store
                .reconcile_sidecar_internal_from_groups(gid, &groups)),
            Err(e) => {
                warn!(error = %e, "list_groups failed during sidecar reconcile");
                Ok(None)
            }
        }
    }

    #[instrument(skip(self, message))]
    async fn handle_relay(&self, message: &BotMessage) -> AppResult<()> {
        if self.bot_identity.is_bot_message(message) {
            debug!("Skipping bot-authored message for relay");
            return Ok(());
        }

        let Some(gid) = message.group_id.as_deref() else {
            return Ok(());
        };

        if let Some((main_id, lang)) = self.resolve_sidecar_route(message).await? {
            if !self.store.allow_message(&main_id) {
                warn!(main_id, "Rate limit: skipping sidecar fan-out");
                return Ok(());
            }
            return self.handle_sidecar_in(message, &main_id, &lang).await;
        }

        if let Some(bridge) = self.store.get_bridge(gid) {
            if bridge.sidecars.is_empty() {
                return Ok(());
            }
            if !self.store.allow_message(gid) {
                warn!(main_id = gid, "Rate limit: skipping main fan-out");
                return Ok(());
            }
            return self.handle_main_out(message, &bridge).await;
        }

        Ok(())
    }

    /// Subscribe `user` (defaults to message.source) to a language sidecar on `main_id`.
    pub(crate) async fn subscribe_user_to_thread(
        store: &Arc<GroupPreferencesStore>,
        signal: &Arc<SignalClient>,
        message: &BotMessage,
        main_id: &str,
        lang_token: &str,
        user_override: Option<&str>,
        address_override: Option<&str>,
    ) -> AppResult<String> {
        let Some(lang) = resolve_language(lang_token) else {
            return Ok(format!(
                "Unknown language `{lang_token}`. Try !list-langs for supported codes."
            ));
        };

        let address = match address_override
            .map(str::to_string)
            .or_else(|| message.invite_address())
        {
            Some(a) => a,
            None => return Ok(NO_ADDRESS_MSG.into()),
        };

        let user_key = user_override
            .map(str::to_string)
            .unwrap_or_else(|| message.source.clone());
        let bot = &message.receiving_account;

        if let Some(existing) = store.member_lang(main_id, &user_key) {
            if existing == lang.code {
                return Ok(format!(
                "You are already in the {} sidecar. Accept the Signal invite if it is still pending.",
                lang.name
            ));
            }
            if let Some(bridge) = store.get_bridge(main_id) {
                if let Some(old_send) = bridge.sidecar_send_id(&existing) {
                    if let Err(e) = signal
                        .remove_members(bot, old_send, vec![address.clone()])
                        .await
                    {
                        warn!(error = %e, "Failed to remove member from old sidecar");
                    }
                }
            }
        }

        let bridge = store.get_bridge(main_id);
        let sidecar_exists = bridge
            .as_ref()
            .and_then(|b| b.sidecar_send_id(lang.code))
            .is_some();

        if sidecar_exists {
            let send_id = bridge
                .as_ref()
                .and_then(|b| b.sidecar_send_id(lang.code))
                .unwrap()
                .to_string();
            if let Err(e) = signal
                .add_members(bot, &send_id, vec![address.clone()])
                .await
            {
                return Ok(format!(
                    "Could not add you to the {} sidecar: {e}. Try again shortly.",
                    lang.name
                ));
            }
        } else {
            let (name, description, welcome) =
                sidecar_copy(lang, message.group_name.as_deref(), main_id);
            match signal
                .create_group(bot, &name, vec![address.clone()], Some(&description))
                .await
            {
                Ok(group) => {
                    store.set_sidecar(
                        main_id,
                        lang.code,
                        group.id.clone(),
                        group.internal_id.clone(),
                    );
                    if let Err(e) = signal.send(bot, &group.id, &welcome).await {
                        warn!(error = %e, "Failed to send sidecar welcome");
                    }
                }
                Err(e) => {
                    return Ok(format!(
                        "Could not create the {} sidecar: {e}. Try again shortly.",
                        lang.name
                    ));
                }
            }
        }

        store.set_bridge_member(main_id, &user_key, lang.code, Some(address));

        info!(
            main_id,
            lang = lang.code,
            user = %user_key,
            "translate-me-thread: subscribed to sidecar"
        );

        let who = if user_override.is_some_and(|u| u != message.source.as_str()) {
            user_key
        } else {
            message.display_name()
        };
        Ok(format!("{who} joined {} thread", lang.name))
    }

    async fn handle_main_out(
        &self,
        message: &BotMessage,
        bridge: &crate::group_preferences_store::LanguageBridge,
    ) -> AppResult<()> {
        let spoken = strip_transcript_prefix(&message.text, DEFAULT_TRANSCRIPT_PREFIX);
        let detected = detect_text_language(&spoken);
        let display = message.display_name();
        let bot = &message.receiving_account;
        let mut translation_cache: HashMap<String, String> = HashMap::new();

        for (lang, send_id) in &bridge.sidecars {
            let Some(target_lang) = resolve_language(lang) else {
                warn!(lang, "Unknown sidecar language code; skipping");
                continue;
            };
            let body = if detected.as_deref() == Some(lang.as_str()) {
                spoken.clone()
            } else if let Some(cached) = translation_cache.get(lang) {
                cached.clone()
            } else {
                match near_ai_translate(&self.near_ai, &spoken, target_lang).await {
                    Ok(t) => {
                        translation_cache.insert(lang.clone(), t.clone());
                        t
                    }
                    Err(e) => {
                        warn!(error = %e, target = %lang, "Main→sidecar translate failed");
                        continue;
                    }
                }
            };
            let formatted = format_attribution(&display, &body);
            if let Err(e) = self.signal.send(bot, send_id, &formatted).await {
                warn!(error = %e, send_id, "Failed to send main→sidecar");
            }
        }
        Ok(())
    }

    async fn handle_sidecar_in(
        &self,
        message: &BotMessage,
        main_id: &str,
        source_lang: &str,
    ) -> AppResult<()> {
        let Some(bridge) = self.store.get_bridge(main_id) else {
            return Ok(());
        };

        let spoken = strip_transcript_prefix(&message.text, DEFAULT_TRANSCRIPT_PREFIX);
        let display = message.display_name();
        let bot = &message.receiving_account;
        let to_main = format_attribution(&display, &spoken);

        // Resolve main send id (incoming group_id is internal).
        let main_recipient = match self
            .signal
            .resolve_group_send_id_for_account(bot, main_id)
            .await
        {
            Ok(id) => id,
            Err(e) => {
                warn!(error = %e, main_id, "Could not resolve main send id");
                return Ok(());
            }
        };

        if let Err(e) = self.signal.send(bot, &main_recipient, &to_main).await {
            warn!(error = %e, "Failed to relay sidecar→main");
        }

        let mut translation_cache: HashMap<String, String> = HashMap::new();
        for (lang, send_id) in &bridge.sidecars {
            if lang == source_lang {
                continue;
            }
            let Some(target_lang) = resolve_language(lang) else {
                warn!(lang, "Unknown sidecar language code; skipping");
                continue;
            };
            let body = if let Some(cached) = translation_cache.get(lang) {
                cached.clone()
            } else {
                match near_ai_translate(&self.near_ai, &spoken, target_lang).await {
                    Ok(t) => {
                        translation_cache.insert(lang.clone(), t.clone());
                        t
                    }
                    Err(e) => {
                        warn!(error = %e, target = %lang, "Sidecar→sidecar translate failed");
                        continue;
                    }
                }
            };
            let formatted = format_attribution(&display, &body);
            if let Err(e) = self.signal.send(bot, send_id, &formatted).await {
                warn!(error = %e, send_id, "Failed to send sidecar→sidecar");
            }
        }
        Ok(())
    }
}

fn format_attribution(display_name: &str, body: &str) -> String {
    format!("{display_name}:\n{body}")
}

const DISAMBIGUATOR_MAX: usize = 24;

/// English title / description / welcome for a new Language Thread sidecar.
fn sidecar_copy(
    lang: &Language,
    main_group_name: Option<&str>,
    main_id: &str,
) -> (String, String, String) {
    let disambiguator = sidecar_disambiguator(main_group_name, main_id);
    let name = format!("{} · {}", lang.name, disambiguator);
    let description = format!(
        "{} Language Thread bridged to the main group ({}).",
        lang.name, disambiguator
    );
    let welcome = format!(
        "Welcome to {name}. Messages here are bridged with the main group.

!commands
!rename <name>
!leave
!info"
    );
    (name, description, welcome)
}

fn sidecar_disambiguator(main_group_name: Option<&str>, main_id: &str) -> String {
    if let Some(label) = truncate_main_label(main_group_name) {
        return label;
    }
    short_main_id_hash(main_id)
}

fn truncate_main_label(name: Option<&str>) -> Option<String> {
    let raw = name?.trim();
    if raw.is_empty() {
        return None;
    }
    let collapsed: String = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return None;
    }
    let truncated: String = collapsed.chars().take(DISAMBIGUATOR_MAX).collect();
    let trimmed = truncated.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn short_main_id_hash(main_id: &str) -> String {
    // FNV-1a 32-bit — stable, no extra deps, enough for a 4-hex chat-list suffix.
    let mut hash: u32 = 0x811c_9dc5;
    for b in main_id.as_bytes() {
        hash ^= u32::from(*b);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    format!("{:04x}", hash & 0xffff)
}

fn starts_with_word(text: &str, prefix: &str) -> bool {
    text == prefix
        || text
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(' '))
}

fn strip_word_prefix<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    if text == prefix {
        return Some("");
    }
    text.strip_prefix(prefix)
        .filter(|rest| rest.is_empty() || rest.starts_with(' '))
        .map(str::trim)
}

#[async_trait]
impl CommandHandler for TranslateMeHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        if self.bot_identity.is_bot_message(message) {
            return false;
        }
        Self::is_command(&message.text) || self.is_relay_candidate(message)
    }

    fn handles_own_reply(&self) -> bool {
        true
    }

    fn label(&self) -> &'static str {
        "translate_me"
    }

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        if Self::is_command(&message.text) {
            let reply = self.handle_command(message).await?;
            if !reply.is_empty() {
                if let Err(e) = self.signal.reply(message, &reply).await {
                    warn!(error = %e, "Failed to send translate-me command reply");
                }
            }
            return Ok(String::new());
        }

        self.handle_relay(message).await?;
        Ok(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::translate_all::TranslateAllHandler;
    use crate::group_preferences_store::GroupTranslateMode;

    fn group_msg(source: &str, text: &str) -> BotMessage {
        BotMessage {
            source: source.into(),
            source_number: Some(source.into()),
            source_name: Some("Maria".into()),
            text: text.into(),
            timestamp: 1,
            message_timestamp: 1,
            is_group: true,
            group_id: Some("main-internal".into()),
            group_name: None,
            receiving_account: "+15550001111".into(),
            attachments: vec![],
            quote: None,
        }
    }

    #[test]
    fn sidecar_copy_uses_main_group_name() {
        let it = resolve_language("it").unwrap();
        let (name, description, welcome) = sidecar_copy(it, Some("  Stacked  "), "main-id");
        assert_eq!(name, "Italian · Stacked");
        assert!(description.contains("Stacked"));
        assert!(welcome.starts_with("Welcome to Italian · Stacked"));
        assert!(welcome.contains("!commands\n!rename <name>\n!leave\n!info"));
    }

    #[test]
    fn sidecar_copy_falls_back_to_hash_without_group_name() {
        let es = resolve_language("es").unwrap();
        let (name, _, _) = sidecar_copy(es, None, "main-internal-abc");
        assert!(name.starts_with("Spanish · "));
        assert!(!name.contains("None"));
        let suffix = name.rsplit('·').next().unwrap().trim();
        assert_eq!(suffix.len(), 4);
        assert!(suffix.chars().all(|c| c.is_ascii_hexdigit()));

        let (name2, _, _) = sidecar_copy(es, Some("   "), "main-internal-abc");
        assert_eq!(name, name2);
    }

    #[test]
    fn sidecar_copy_truncates_long_main_names() {
        let en = resolve_language("en").unwrap();
        let long = "A".repeat(40);
        let (name, _, _) = sidecar_copy(en, Some(&long), "main");
        let label = name.rsplit('·').next().unwrap().trim();
        assert_eq!(label.chars().count(), DISAMBIGUATOR_MAX);
    }

    #[test]
    fn matches_on_off_commands() {
        assert!(TranslateMeHandler::is_on_command("!translate-me-thread es"));
        assert!(TranslateMeHandler::is_on_command("!translate-me-thread es"));
        assert!(TranslateMeHandler::is_off_command("!leave"));
        assert!(TranslateMeHandler::is_off_command("!leave"));
        assert!(!TranslateMeHandler::is_command("!translate-on es en"));
        assert!(!TranslateMeHandler::is_command("!translate-me-on es en"));
        assert!(!TranslateMeHandler::is_command("!translate es"));
    }

    #[test]
    fn parses_lang_arg() {
        assert_eq!(
            TranslateMeHandler::on_lang_arg("!translate-me-thread es"),
            Some("es")
        );
        assert_eq!(
            TranslateMeHandler::on_lang_arg("!translate-me-thread en"),
            Some("en")
        );
        assert_eq!(
            TranslateMeHandler::on_lang_arg("!translate-me-thread es"),
            Some("es")
        );
        assert_eq!(
            TranslateMeHandler::on_lang_arg("!translate-me-thread"),
            None
        );
    }

    #[test]
    fn attribution_format() {
        assert_eq!(format_attribution("Maria", "Hola"), "Maria:\nHola");
    }

    #[test]
    fn display_name_prefers_source_name() {
        let m = group_msg("+15550002222", "hi");
        assert_eq!(m.display_name(), "Maria");
    }

    #[test]
    fn bot_messages_do_not_match() {
        let store = GroupPreferencesStore::new_in_memory(0);
        store.set_sidecar(
            "main-internal",
            "es",
            "group.es".into(),
            "es-internal".into(),
        );
        let identity = BotIdentity::new();
        identity.remember_phone("+15550001111");

        // Handler constructed without live clients — only matches() needs identity+store.
        // Use NearAi/Signal stubs via wiremock in integration tests; here test identity gate
        // with a minimal fake by checking is_bot_message path directly.
        let bot_msg = BotMessage {
            source: "+15550001111".into(),
            source_number: Some("+15550001111".into()),
            source_name: None,
            text: "relayed".into(),
            timestamp: 1,
            message_timestamp: 1,
            is_group: true,
            group_id: Some("main-internal".into()),
            group_name: None,
            receiving_account: "+15550001111".into(),
            attachments: vec![],
            quote: None,
        };
        assert!(identity.is_bot_message(&bot_msg));
    }

    #[tokio::test]
    async fn execute_command_paths_without_creating_sidecar() {
        use serde_json::json;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let signal = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(3)
            .mount(&signal)
            .await;

        let store = GroupPreferencesStore::new_in_memory(0);
        let identity = BotIdentity::new();
        let handler = TranslateMeHandler::new(
            store,
            Arc::new(
                NearAiClient::new(
                    "key",
                    "http://127.0.0.1:9",
                    "m",
                    std::time::Duration::from_secs(2),
                )
                .unwrap(),
            ),
            Arc::new(SignalClient::new(signal.uri()).unwrap()),
            identity,
        );

        let dm = BotMessage {
            source: "+15550002222".into(),
            source_number: Some("+15550002222".into()),
            source_name: None,
            text: "!translate-me-thread es".into(),
            timestamp: 1,
            message_timestamp: 1,
            is_group: false,
            group_id: None,
            group_name: None,
            receiving_account: "+15550001111".into(),
            attachments: vec![],
            quote: None,
        };
        assert!(handler.matches(&dm));
        assert!(handler.execute(&dm).await.unwrap().is_empty());

        let mut group = dm.clone();
        group.is_group = true;
        group.group_id = Some("group.main".into());
        group.text = "!translate-me-thread".into();
        assert!(handler.execute(&group).await.unwrap().is_empty());

        group.text = "!leave".into();
        assert!(handler.execute(&group).await.unwrap().is_empty());
    }

    fn handler_pair(
        store: Arc<GroupPreferencesStore>,
        signal_uri: String,
        near_uri: String,
    ) -> TranslateMeHandler {
        TranslateMeHandler::new(
            store,
            Arc::new(
                NearAiClient::new("key", near_uri, "m", std::time::Duration::from_secs(5)).unwrap(),
            ),
            Arc::new(SignalClient::new(signal_uri).unwrap()),
            BotIdentity::new(),
        )
    }

    #[tokio::test]
    async fn create_sidecar_on_existing_add_switch_and_off() {
        use serde_json::json;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use wiremock::matchers::{method, path, path_regex};
        use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

        struct CreateGroupResponder {
            count: Arc<AtomicUsize>,
        }
        impl Respond for CreateGroupResponder {
            fn respond(&self, _request: &Request) -> ResponseTemplate {
                // Call order: first subscribe creates es, language switch creates fr.
                let n = self.count.fetch_add(1, Ordering::SeqCst);
                let id = if n == 0 { "group.es" } else { "group.fr" };
                ResponseTemplate::new(200).set_body_json(json!({"id": id}))
            }
        }

        let signal = MockServer::start().await;
        let near = MockServer::start().await;
        mount_near(&near).await;
        Mock::given(method("POST"))
            .and(path("/v2/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&signal)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/groups/%2B15550001111"))
            .respond_with(CreateGroupResponder {
                count: Arc::new(AtomicUsize::new(0)),
            })
            .mount(&signal)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/groups/%2B15550001111"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {
                    "name": "Language Thread Spanish",
                    "id": "group.es",
                    "internal_id": "es-internal"
                },
                {
                    "name": "Language Thread French",
                    "id": "group.fr",
                    "internal_id": "fr-internal"
                }
            ])))
            .mount(&signal)
            .await;
        Mock::given(method("POST"))
            .and(path_regex(r"^/v1/groups/%2B15550001111/.+/members$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&signal)
            .await;
        Mock::given(method("DELETE"))
            .and(path_regex(r"^/v1/groups/%2B15550001111/.+/members$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&signal)
            .await;

        let store = GroupPreferencesStore::new_in_memory(0);
        let handler = handler_pair(store.clone(), signal.uri(), near.uri());

        let mut msg = group_msg("+15550002222", "!translate-me-thread es");
        msg.group_id = Some("main-internal".into());
        assert!(handler.matches(&msg));
        assert!(handler.execute(&msg).await.unwrap().is_empty());
        assert_eq!(
            store
                .member_lang("main-internal", "+15550002222")
                .as_deref(),
            Some("es")
        );

        // Already subscribed same language.
        assert!(handler.execute(&msg).await.unwrap().is_empty());

        // Existing sidecar: second user joins via add_members.
        let mut other = group_msg("+15550003333", "!translate-me-thread es");
        other.group_id = Some("main-internal".into());
        assert!(handler.execute(&other).await.unwrap().is_empty());

        msg.text = "!translate-me-thread fr".into();
        assert!(handler.execute(&msg).await.unwrap().is_empty());
        assert_eq!(
            store
                .member_lang("main-internal", "+15550002222")
                .as_deref(),
            Some("fr")
        );

        // Off from sidecar group.
        msg.group_id = Some("fr-internal".into());
        msg.text = "!leave".into();
        assert!(handler.execute(&msg).await.unwrap().is_empty());
        assert!(store.member_lang("main-internal", "+15550002222").is_none());
    }

    #[tokio::test]
    async fn relay_main_to_sidecar_and_sidecar_to_main() {
        let signal = wiremock::MockServer::start().await;
        let near = wiremock::MockServer::start().await;
        mount_relay_signal(&signal).await;
        mount_near(&near).await;

        let store = GroupPreferencesStore::new_in_memory(0);
        store.set_sidecar(
            "main-internal",
            "es",
            "group.es".into(),
            "es-internal".into(),
        );
        store.set_sidecar(
            "main-internal",
            "fr",
            "group.fr".into(),
            "fr-internal".into(),
        );

        let handler = handler_pair(store, signal.uri(), near.uri());

        let main_msg = group_msg("+15550002222", "Hello friends from the mutual aid group");
        assert!(handler.matches(&main_msg));
        assert!(handler.execute(&main_msg).await.unwrap().is_empty());

        let mut side = group_msg("+15550002222", "Bonjour amis du groupe d entraide");
        side.group_id = Some("fr-internal".into());
        assert!(handler.matches(&side));
        assert!(handler.execute(&side).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn relay_main_to_three_sidecars() {
        let signal = wiremock::MockServer::start().await;
        let near = wiremock::MockServer::start().await;
        mount_relay_signal(&signal).await;
        mount_near(&near).await;

        let store = GroupPreferencesStore::new_in_memory(0);
        for (lang, send, internal) in [
            ("es", "group.es", "es-internal"),
            ("fr", "group.fr", "fr-internal"),
            ("hi", "group.hi", "hi-internal"),
        ] {
            store.set_sidecar("main-internal", lang, send.into(), internal.into());
        }

        let handler = handler_pair(store, signal.uri(), near.uri());
        let main_msg = group_msg(
            "+15550002222",
            "Hello friends from the mutual aid group tonight",
        );
        assert!(handler.execute(&main_msg).await.unwrap().is_empty());

        let recipients = send_recipients(&signal).await;
        assert_eq!(recipients.len(), 3);
        assert!(recipients.contains(&"group.es".to_string()));
        assert!(recipients.contains(&"group.fr".to_string()));
        assert!(recipients.contains(&"group.hi".to_string()));
    }

    #[tokio::test]
    async fn relay_sidecar_skips_source_and_fans_out() {
        let signal = wiremock::MockServer::start().await;
        let near = wiremock::MockServer::start().await;
        mount_relay_signal(&signal).await;
        mount_near(&near).await;

        let store = GroupPreferencesStore::new_in_memory(0);
        for (lang, send, internal) in [
            ("es", "group.es", "es-internal"),
            ("fr", "group.fr", "fr-internal"),
            ("hi", "group.hi", "hi-internal"),
        ] {
            store.set_sidecar("main-internal", lang, send.into(), internal.into());
        }

        let handler = handler_pair(store, signal.uri(), near.uri());
        let mut side = group_msg("+15550002222", "Bonjour amis du groupe d entraide ensemble");
        side.group_id = Some("fr-internal".into());
        assert!(handler.execute(&side).await.unwrap().is_empty());

        let recipients = send_recipients(&signal).await;
        assert!(recipients.contains(&"group.main".to_string()));
        assert!(recipients.contains(&"group.es".to_string()));
        assert!(recipients.contains(&"group.hi".to_string()));
        assert!(!recipients.contains(&"group.fr".to_string()));
        assert_eq!(recipients.len(), 3);
    }

    #[tokio::test]
    async fn relay_bot_authored_zero_sends() {
        let signal = wiremock::MockServer::start().await;
        mount_relay_signal(&signal).await;

        let store = GroupPreferencesStore::new_in_memory(0);
        store.set_sidecar(
            "main-internal",
            "es",
            "group.es".into(),
            "es-internal".into(),
        );
        let identity = BotIdentity::new();
        identity.remember_phone("+15550001111");
        let handler = TranslateMeHandler::new(
            store,
            Arc::new(
                NearAiClient::new(
                    "key",
                    "http://127.0.0.1:9",
                    "m",
                    std::time::Duration::from_secs(2),
                )
                .unwrap(),
            ),
            Arc::new(SignalClient::new(signal.uri()).unwrap()),
            identity,
        );

        let bot_msg = group_msg("+15550001111", "Maria:\nHola");
        assert!(!handler.matches(&bot_msg));
        assert!(handler.execute(&bot_msg).await.unwrap().is_empty());
        assert!(send_recipients(&signal).await.is_empty());
    }

    #[tokio::test]
    async fn lookup_reconciles_wrong_internal_id() {
        let signal = wiremock::MockServer::start().await;
        mount_relay_signal(&signal).await;

        let store = GroupPreferencesStore::new_in_memory(0);
        store.set_sidecar("main-internal", "es", "group.es".into(), "group.es".into());

        let handler = handler_pair(store.clone(), signal.uri(), "http://127.0.0.1:9".into());

        let mut side = group_msg("+15550002222", "Hola desde el thread");
        side.group_id = Some("es-internal".into());
        assert!(handler.execute(&side).await.unwrap().is_empty());
        assert_eq!(
            store.lookup_sidecar("es-internal"),
            Some(("main-internal".into(), "es".into()))
        );
        assert!(send_recipients(&signal)
            .await
            .contains(&"group.main".to_string()));

        assert!(handler.execute(&side).await.unwrap().is_empty());
        let recipients = send_recipients(&signal).await;
        assert!(
            recipients
                .iter()
                .filter(|r| r.as_str() == "group.main")
                .count()
                >= 2
        );
    }

    #[tokio::test]
    async fn relay_after_enable_threads_switch() {
        use serde_json::json;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let signal = MockServer::start().await;
        let near = MockServer::start().await;
        mount_near(&near).await;
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
                    "name": "Main",
                    "id": "group.main",
                    "internal_id": "main-internal"
                },
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
        store.set_member_translate("main-internal", "+15550002222", mode);

        let translate_me = handler_pair(store.clone(), signal.uri(), near.uri());
        let translate_all = TranslateAllHandler::new(
            store.clone(),
            Arc::new(
                NearAiClient::new("key", near.uri(), "m", std::time::Duration::from_secs(5))
                    .unwrap(),
            ),
            Arc::new(SignalClient::new(signal.uri()).unwrap()),
        );

        let mut blocked = group_msg("+15550002222", "!translate-me-thread es");
        blocked.group_id = Some("main-internal".into());
        assert!(translate_me.execute(&blocked).await.unwrap().is_empty());
        assert!(store.get_pending_switch("main-internal").is_some());

        let mut enable = group_msg("+15550002222", "!enable-threads");
        enable.group_id = Some("main-internal".into());
        assert!(translate_all.execute(&enable).await.unwrap().is_empty());
        assert!(store.threads_active("main-internal"));
        assert_eq!(
            store.lookup_sidecar("es-internal"),
            Some(("main-internal".into(), "es".into()))
        );

        let main_msg = group_msg(
            "+15550002222",
            "Hello everyone in the main mutual aid group",
        );
        assert!(translate_me.execute(&main_msg).await.unwrap().is_empty());
        assert!(send_recipients(&signal)
            .await
            .contains(&"group.es".to_string()));

        let mut side = group_msg("+15550002222", "Hola desde el thread español");
        side.group_id = Some("es-internal".into());
        assert!(translate_me.execute(&side).await.unwrap().is_empty());
        assert!(send_recipients(&signal)
            .await
            .contains(&"group.main".to_string()));
    }

    #[tokio::test]
    async fn relay_n1_then_second_sidecar() {
        let signal = wiremock::MockServer::start().await;
        let near = wiremock::MockServer::start().await;
        mount_relay_signal(&signal).await;
        mount_near(&near).await;

        let store = GroupPreferencesStore::new_in_memory(0);
        store.set_sidecar(
            "main-internal",
            "es",
            "group.es".into(),
            "es-internal".into(),
        );
        let handler = handler_pair(store.clone(), signal.uri(), near.uri());

        let main_msg = group_msg("+15550002222", "Hello friends from the mutual aid group");
        assert!(handler.execute(&main_msg).await.unwrap().is_empty());
        let first = send_recipients(&signal).await;
        assert_eq!(first, vec!["group.es".to_string()]);

        store.set_sidecar(
            "main-internal",
            "fr",
            "group.fr".into(),
            "fr-internal".into(),
        );
        // Reset received requests by starting a fresh mock is hard; count delta instead.
        let before = send_recipients(&signal).await.len();
        assert!(handler.execute(&main_msg).await.unwrap().is_empty());
        let after = send_recipients(&signal).await;
        let new_sends = &after[before..];
        assert_eq!(new_sends.len(), 2);
        assert!(new_sends.contains(&"group.es".to_string()));
        assert!(new_sends.contains(&"group.fr".to_string()));
    }

    async fn mount_relay_signal(signal: &wiremock::MockServer) {
        use serde_json::json;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};

        Mock::given(method("POST"))
            .and(path("/v2/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(signal)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/groups/%2B15550001111"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {
                    "name": "Main",
                    "id": "group.main",
                    "internal_id": "main-internal"
                },
                {
                    "name": "ES",
                    "id": "group.es",
                    "internal_id": "es-internal"
                },
                {
                    "name": "FR",
                    "id": "group.fr",
                    "internal_id": "fr-internal"
                },
                {
                    "name": "HI",
                    "id": "group.hi",
                    "internal_id": "hi-internal"
                }
            ])))
            .mount(signal)
            .await;
    }

    async fn mount_near(near: &wiremock::MockServer) {
        use serde_json::json;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "1",
                "choices": [{"index": 0, "message": {"role": "assistant", "content": "Translated"}, "finish_reason": "stop"}],
                "created": 1,
                "model": "m",
                "object": "chat.completion"
            })))
            .mount(near)
            .await;
    }

    async fn send_recipients(signal: &wiremock::MockServer) -> Vec<String> {
        let mut out = Vec::new();
        let Some(requests) = signal.received_requests().await else {
            return out;
        };
        for req in requests {
            if req.url.path() != "/v2/send" {
                continue;
            }
            let body: serde_json::Value =
                serde_json::from_slice(&req.body).unwrap_or(serde_json::json!({}));
            if let Some(arr) = body["recipients"].as_array() {
                for r in arr {
                    if let Some(s) = r.as_str() {
                        out.push(s.to_string());
                    }
                }
            }
        }
        out
    }

    async fn send_messages(signal: &wiremock::MockServer) -> Vec<String> {
        let mut out = Vec::new();
        let Some(requests) = signal.received_requests().await else {
            return out;
        };
        for req in requests {
            if req.url.path() != "/v2/send" {
                continue;
            }
            let body: serde_json::Value =
                serde_json::from_slice(&req.body).unwrap_or(serde_json::json!({}));
            if let Some(msg) = body["message"].as_str() {
                out.push(msg.to_string());
            }
        }
        out
    }

    #[tokio::test]
    async fn enable_in_chat_notifies_each_sidecar_before_removing_members() {
        use serde_json::json;
        use wiremock::matchers::{method, path, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let signal = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&signal)
            .await;
        Mock::given(method("DELETE"))
            .and(path_regex(r"^/v1/groups/%2B15550001111/.+/members$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&signal)
            .await;

        let store = GroupPreferencesStore::new_in_memory(0);
        store.set_sidecar(
            "main-internal",
            "es",
            "group.es".into(),
            "es-internal".into(),
        );
        store.set_sidecar(
            "main-internal",
            "fr",
            "group.fr".into(),
            "fr-internal".into(),
        );
        store.set_bridge_member(
            "main-internal",
            "+15550002222",
            "es",
            Some("+15550002222".into()),
        );
        store.set_bridge_member(
            "main-internal",
            "+15550003333",
            "fr",
            Some("+15550003333".into()),
        );

        let handler = handler_pair(store.clone(), signal.uri(), "http://127.0.0.1:9".into());
        let mut msg = group_msg("+15550001111", "!enable-in-chat");
        msg.group_id = Some("main-internal".into());
        msg.source = "+15550001111".into();

        let reply = handler.execute(&msg).await.unwrap();
        assert!(reply.is_empty());
        assert!(store.get_bridge("main-internal").is_none());

        let recipients = send_recipients(&signal).await;
        assert!(recipients.contains(&"group.es".to_string()));
        assert!(recipients.contains(&"group.fr".to_string()));

        let bodies = send_messages(&signal).await;
        assert_eq!(bodies.len(), 2);
        assert!(bodies
            .iter()
            .all(|m| m.contains("Language Threads were disabled")));
        assert!(bodies.iter().all(|m| m.contains("Return to the main chat")));
    }

    #[tokio::test]
    async fn on_rejects_unknown_lang_and_missing_address() {
        use serde_json::json;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let signal = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&signal)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/groups/%2B15550001111"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
                "name": "Main",
                "id": "group.main",
                "internal_id": "main-internal"
            }, {
                "name": "ES",
                "id": "group.es",
                "internal_id": "es-internal"
            }])))
            .mount(&signal)
            .await;

        let store = GroupPreferencesStore::new_in_memory(0);
        store.set_sidecar(
            "main-internal",
            "es",
            "group.es".into(),
            "es-internal".into(),
        );
        let handler = handler_pair(store, signal.uri(), "http://127.0.0.1:9".into());

        let unknown = group_msg("+15550002222", "!translate-me-thread zz");
        assert!(handler.execute(&unknown).await.unwrap().is_empty());

        let mut no_addr = group_msg("alice", "!translate-me-thread es");
        no_addr.source_number = None;
        assert!(handler.execute(&no_addr).await.unwrap().is_empty());

        // Subscribe from sidecar is rejected.
        let mut from_side = group_msg("+15550002222", "!translate-me-thread es");
        from_side.group_id = Some("es-internal".into());
        assert!(handler.execute(&from_side).await.unwrap().is_empty());
    }
}
