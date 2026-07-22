//! Language sidecar bridge: `!translate-me-on` / `!translate-me-off` + relay engine.
//!
//! Main group stays bilingual. Each subscribed language gets a `BAM {Language}`
//! Signal sidecar. Messages fan out: main→sidecars (relay/translate),
//! sidecar→main (relay) + other sidecars (translate). Bot never relays itself.

use crate::bot_identity::BotIdentity;
use crate::commands::translate_lang::resolve_language;
use crate::commands::translate_service::{detect_text_language, near_ai_translate};
use crate::commands::CommandHandler;
use crate::error::AppResult;
use crate::group_preferences_store::GroupPreferencesStore;
use async_trait::async_trait;
use near_ai_client::NearAiClient;
use signal_client::{BotMessage, SignalClient};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

const GROUP_ONLY_MSG: &str =
    "!translate-me-on is only available in the main mutual-aid group (not DMs).";
const SIDECAR_ON_MSG: &str =
    "Subscribe from the main group with !translate-me-on <lang>. Use !translate-me-off here to leave.";
const USAGE_MSG: &str =
    "Usage: !translate-me-on <lang> (e.g. !translate-me-on es), or !translate-me-off";
const NO_ADDRESS_MSG: &str = "Could not invite you: Signal did not include your phone number. \
Message this bot in a 1:1 chat once, then retry !translate-me-on <lang>.";

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
        starts_with_word(t, "!translate-me-on")
            || starts_with_word(t, "!translation-me-on")
            || is_translate_me_with_rest(t, "on")
    }

    fn is_off_command(text: &str) -> bool {
        let t = text.trim();
        starts_with_word(t, "!translate-me-off")
            || starts_with_word(t, "!translation-me-off")
            || is_translate_me_with_rest(t, "off")
            || t == "!translate-me off"
            || t == "!translation-me off"
    }

    fn is_command(text: &str) -> bool {
        let t = text.trim();
        Self::is_on_command(t)
            || Self::is_off_command(t)
            || t == "!translate-me"
            || t == "!translation-me"
            || starts_with_word(t, "!translate-me ")
            || starts_with_word(t, "!translation-me ")
    }

    fn on_lang_arg(text: &str) -> Option<&str> {
        let t = text.trim();
        for prefix in ["!translate-me-on", "!translation-me-on"] {
            if let Some(rest) = strip_word_prefix(t, prefix) {
                return rest.split_whitespace().next();
            }
        }
        for prefix in ["!translate-me", "!translation-me"] {
            if let Some(rest) = strip_word_prefix(t, prefix) {
                let mut parts = rest.split_whitespace();
                match parts.next() {
                    Some("on") => return parts.next(),
                    Some(token) if resolve_language(token).is_some() => return Some(token),
                    _ => return None,
                }
            }
        }
        None
    }

    fn is_relay_candidate(&self, message: &BotMessage) -> bool {
        let text = message.text.trim();
        if message.group_id.is_none() || message.is_voice_note() || text.is_empty() || text.starts_with('!')
        {
            return false;
        }
        let Some(gid) = message.group_id.as_deref() else {
            return false;
        };
        self.store.get_bridge(gid).is_some() || self.store.lookup_sidecar(gid).is_some()
    }

    async fn handle_command(&self, message: &BotMessage) -> AppResult<String> {
        let text = message.text.trim();

        if Self::is_off_command(text) {
            return self.handle_off(message).await;
        }

        if Self::is_on_command(text) || starts_with_word(text, "!translate-me ") {
            let Some(gid) = message.group_id.as_deref() else {
                return Ok(GROUP_ONLY_MSG.into());
            };

            if self.store.lookup_sidecar(gid).is_some() {
                return Ok(SIDECAR_ON_MSG.into());
            }

            let Some(lang_token) = Self::on_lang_arg(text) else {
                return Ok(USAGE_MSG.into());
            };
            return self.handle_on(message, gid, lang_token).await;
        }

        Ok(USAGE_MSG.into())
    }

    async fn handle_on(
        &self,
        message: &BotMessage,
        main_id: &str,
        lang_token: &str,
    ) -> AppResult<String> {
        let Some(lang) = resolve_language(lang_token) else {
            return Ok(format!(
                "Unknown language `{lang_token}`. Try !list-langs for supported codes."
            ));
        };

        let Some(address) = message.invite_address() else {
            return Ok(NO_ADDRESS_MSG.into());
        };

        let user_key = message.source.clone();
        let bot = &message.receiving_account;

        if let Some(existing) = self.store.member_lang(main_id, &user_key) {
            if existing == lang.code {
                return Ok(format!(
                    "You are already in the {} sidecar. Accept the Signal invite if it is still pending.",
                    lang.name
                ));
            }
            // Language switch: remove from old sidecar first.
            if let Some(bridge) = self.store.get_bridge(main_id) {
                if let Some(old_send) = bridge.sidecar_send_id(&existing) {
                    if let Err(e) = self
                        .signal
                        .remove_members(bot, old_send, vec![address.clone()])
                        .await
                    {
                        warn!(error = %e, "Failed to remove member from old sidecar");
                    }
                }
            }
        }

        let bridge = self.store.get_bridge(main_id);
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
            if let Err(e) = self
                .signal
                .add_members(bot, &send_id, vec![address.clone()])
                .await
            {
                return Ok(format!(
                    "Could not add you to the {} sidecar: {e}. Try again shortly.",
                    lang.name
                ));
            }
        } else {
            let name = format!("BAM {}", lang.name);
            let description = format!(
                "{} sidecar bridged to the main mutual-aid group.",
                lang.name
            );
            match self
                .signal
                .create_group(
                    bot,
                    &name,
                    vec![address.clone()],
                    Some(&description),
                )
                .await
            {
                Ok(group) => {
                    self.store.set_sidecar(
                        main_id,
                        lang.code,
                        group.id.clone(),
                        group.internal_id.clone(),
                    );
                    let welcome = format!(
                        "Welcome to BAM {}. Messages here are bridged with the main group.",
                        lang.name
                    );
                    if let Err(e) = self.signal.send(bot, &group.id, &welcome).await {
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

        self.store.set_bridge_member(
            main_id,
            &user_key,
            lang.code,
            Some(address),
        );

        info!(
            main_id,
            lang = lang.code,
            user = %user_key,
            "translate-me-on: subscribed to sidecar"
        );

        Ok(format!(
            "Joined the {} sidecar (BAM {}). Accept the Signal group invite if prompted. \
Use !translate-me-off to leave.",
            lang.name, lang.name
        ))
    }

    async fn handle_off(&self, message: &BotMessage) -> AppResult<String> {
        let Some(gid) = message.group_id.as_deref() else {
            return Ok("!translate-me-off is only available in group chats.".into());
        };

        let (main_id, _) = if let Some(pair) = self.store.lookup_sidecar(gid) {
            pair
        } else if self.store.get_bridge(gid).is_some()
            || self.store.member_lang(gid, &message.source).is_some()
        {
            (gid.to_string(), String::new())
        } else {
            return Ok("You are not subscribed to a language sidecar in this chat.".into());
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
                    warn!(error = %e, "Failed to remove member from sidecar on off");
                }
            }
        }

        let lang_name = resolve_language(&lang)
            .map(|l| l.name)
            .unwrap_or(lang.as_str());
        Ok(format!("Left the {lang_name} sidecar."))
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

        if let Some((main_id, lang)) = self.store.lookup_sidecar(gid) {
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

    async fn handle_main_out(
        &self,
        message: &BotMessage,
        bridge: &crate::group_preferences_store::LanguageBridge,
    ) -> AppResult<()> {
        let detected = detect_text_language(&message.text);
        let display = message.display_name();
        let bot = &message.receiving_account;
        let mut translation_cache: HashMap<String, String> = HashMap::new();

        for (lang, send_id) in &bridge.sidecars {
            let Some(target_lang) = resolve_language(lang) else {
                warn!(lang, "Unknown sidecar language code; skipping");
                continue;
            };
            let body = if detected.as_deref() == Some(lang.as_str()) {
                message.text.clone()
            } else if let Some(cached) = translation_cache.get(lang) {
                cached.clone()
            } else {
                match near_ai_translate(&self.near_ai, &message.text, target_lang).await {
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

        let display = message.display_name();
        let bot = &message.receiving_account;
        let to_main = format_attribution(&display, &message.text);

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
                match near_ai_translate(&self.near_ai, &message.text, target_lang).await {
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

fn is_translate_me_with_rest(text: &str, rest_first: &str) -> bool {
    for prefix in ["!translate-me", "!translation-me"] {
        if let Some(rest) = strip_word_prefix(text, prefix) {
            let mut parts = rest.split_whitespace();
            if parts.next() == Some(rest_first) {
                return true;
            }
        }
    }
    false
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

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        if Self::is_command(&message.text) {
            let reply = self.handle_command(message).await?;
            if !reply.is_empty() {
                if let Err(e) = self
                    .signal
                    .reply(message, &reply)
                    .await
                {
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
    fn matches_on_off_commands() {
        assert!(TranslateMeHandler::is_on_command("!translate-me-on es"));
        assert!(TranslateMeHandler::is_on_command("!translate-me on es"));
        assert!(TranslateMeHandler::is_off_command("!translate-me-off"));
        assert!(TranslateMeHandler::is_off_command("!translate-me off"));
        assert!(!TranslateMeHandler::is_command("!translate-on es en"));
        assert!(!TranslateMeHandler::is_command("!translate es"));
    }

    #[test]
    fn parses_lang_arg() {
        assert_eq!(
            TranslateMeHandler::on_lang_arg("!translate-me-on es"),
            Some("es")
        );
        assert_eq!(
            TranslateMeHandler::on_lang_arg("!translate-me on en"),
            Some("en")
        );
        assert_eq!(
            TranslateMeHandler::on_lang_arg("!translate-me es"),
            Some("es")
        );
        assert_eq!(TranslateMeHandler::on_lang_arg("!translate-me-on"), None);
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
}
