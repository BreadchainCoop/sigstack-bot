//! Language Threads and Bilingual Threads: `!translate-me-thread` / `!leave` /
//! `!enable-in-chat` + relay engine.
//!
//! Language Threads: main stays multilingual; N sidecars; sidecar→main is relay-only.
//! Bilingual Threads (`!translate-me-thread es en`): main=`es`, one sidecar=`en`;
//! both directions translate into the destination room's assigned language.
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
    "Subscribe from the main group with !translate-me-thread <lang> or !translate-me-thread <main> <thread>. Use !leave here to leave.";
const USAGE_MSG: &str = "Usage: !translate-me-thread <lang> (Language Threads) or !translate-me-thread <main> <thread> (Bilingual Threads)\nExamples: !translate-me-thread es\n          !translate-me-thread es en";
const SAME_LANG_MSG: &str = "Choose two different languages. Example: !translate-me-thread es en";
const NO_ADDRESS_MSG: &str = "Could not invite you: Signal did not include your phone number. \
Message this bot in a 1:1 chat once, then retry !translate-me-thread <lang>.";
const LEAVE_SIDECAR_ONLY_MSG: &str =
    "!leave is only available inside a Language Thread. Open that chat and send !leave (or !commands).";
const IN_CHAT_BLOCK_MSG: &str = "In-chat auto-translate is already on in this group, so Language Threads and Bilingual Threads can't start alongside it.\n\nThe three products — in-chat auto, Language Threads, and Bilingual Threads — cannot run at the same time.\n\nTo switch, send:\n!enable-threads";
const LANGUAGE_THREADS_TWO_ARG_REFUSE: &str = "Language Threads is already on (multilingual hub). Tear down with !enable-in-chat before starting Bilingual Threads.";
/// Tear down Language Threads so in-chat can run (`!enable-in-chat`).
pub(crate) const ENABLE_IN_CHAT_CMDS: &[&str] = &[
    "!enable-in-chat",
    "!translation-enable-in-chat",
    "!enable-inchat",
    "!enable in-chat",
];
const THREADS_DISABLED_SIDECAR_MSG: &str = "Language Threads were disabled in the main group (in-chat translation is on).\n\nReturn to the main chat to continue — this thread will no longer relay messages.";
const BILINGUAL_DISABLED_SIDECAR_MSG: &str = "Bilingual Threads were disabled in the main group (in-chat translation is on).\n\nReturn to the main chat to continue — this thread will no longer relay messages.";
pub(crate) const LEAVE_CMDS: &[&str] = &["!leave"];
pub(crate) const THREAD_ON_PREFIXES: &[&str] = &[
    "!translate-me-thread",
    "!translation-me-thread",
    "!translate-me-threads",
    "!translation-me-threads",
];

#[derive(Debug, Clone, PartialEq, Eq)]
enum ThreadCmdArgs {
    Language { lang: String },
    Bilingual { main: String, thread: String },
}

impl ThreadCmdArgs {
    fn validate_languages(&self) -> Result<(), String> {
        match self {
            Self::Language { lang } => {
                if resolve_language(lang).is_none() {
                    return Err(unknown_lang_msg(lang));
                }
            }
            Self::Bilingual { main, thread } => {
                let a = resolve_language(main).ok_or_else(|| unknown_lang_msg(main))?;
                let b = resolve_language(thread).ok_or_else(|| unknown_lang_msg(thread))?;
                if a.code == b.code {
                    return Err(SAME_LANG_MSG.into());
                }
            }
        }
        Ok(())
    }

    fn to_pending(&self, message: &BotMessage) -> PendingSwitch {
        match self {
            Self::Language { lang } => PendingSwitch::EnableThreads {
                user: message.source.clone(),
                lang: resolved_code(lang),
                address: message.invite_address(),
            },
            Self::Bilingual { main, thread } => PendingSwitch::EnableBilingualThreads {
                user: message.source.clone(),
                main_lang: resolved_code(main),
                thread_lang: resolved_code(thread),
                address: message.invite_address(),
            },
        }
    }
}

fn unknown_lang_msg(token: &str) -> String {
    format!("Unknown language `{token}`. Try !list-langs for supported codes.")
}

fn resolved_code(token: &str) -> String {
    resolve_language(token)
        .map(|l| l.code.to_string())
        .unwrap_or_else(|| token.to_string())
}

fn lang_display(code: &str) -> String {
    resolve_language(code)
        .map(|l| l.name.to_string())
        .unwrap_or_else(|| code.to_string())
}

fn bilingual_pair_names(bridge: &LanguageBridge) -> (String, String) {
    (
        lang_display(bridge.main_lang.as_deref().unwrap_or("")),
        lang_display(bridge.bilingual_thread_lang().unwrap_or("")),
    )
}

fn bilingual_main_lang_confirm(bridge: &LanguageBridge) -> String {
    let (main, thread) = bilingual_pair_names(bridge);
    let thread_code = bridge.bilingual_thread_lang().unwrap_or("");
    let main_code = bridge.main_lang.as_deref().unwrap_or("");
    format!(
        "This group's main chat is {main}. The bridged thread is {thread} — send !translate-me-thread {thread_code} (or !translate-me-thread {main_code} {thread_code}) to join it."
    )
}

fn bilingual_third_lang_refuse(bridge: &LanguageBridge) -> String {
    let (main, thread) = bilingual_pair_names(bridge);
    let thread_code = bridge.bilingual_thread_lang().unwrap_or("");
    let main_code = bridge.main_lang.as_deref().unwrap_or("");
    format!(
        "Bilingual Threads is locked to {main} ↔ {thread}. A third language isn't supported. Join the {thread} thread with !translate-me-thread {thread_code} (or !translate-me-thread {main_code} {thread_code})."
    )
}

fn bilingual_different_pair_refuse(bridge: &LanguageBridge) -> String {
    let (main, thread) = bilingual_pair_names(bridge);
    let thread_code = bridge.bilingual_thread_lang().unwrap_or("");
    let main_code = bridge.main_lang.as_deref().unwrap_or("");
    format!(
        "Bilingual Threads is locked to {main} ↔ {thread}. A different pair isn't supported. Tear down with !enable-in-chat, or join with !translate-me-thread {main_code} {thread_code}."
    )
}

#[derive(Clone)]
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

    pub(crate) fn is_on_command(text: &str) -> bool {
        signal_bot_core::starts_with_word_any(text, THREAD_ON_PREFIXES)
    }

    fn is_off_command(text: &str) -> bool {
        signal_bot_core::is_exact_command_any(text, LEAVE_CMDS)
    }

    fn is_enable_in_chat(text: &str) -> bool {
        signal_bot_core::is_exact_command_any(text, ENABLE_IN_CHAT_CMDS)
    }

    fn is_command(text: &str) -> bool {
        let t = text.trim();
        Self::is_on_command(t) || Self::is_off_command(t) || Self::is_enable_in_chat(t)
    }

    fn thread_tokens(text: &str) -> Option<Vec<&str>> {
        let rest = signal_bot_core::strip_prefix_list(text, THREAD_ON_PREFIXES)?;
        Some(rest.split_whitespace().collect())
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

            let Some(tokens) = Self::thread_tokens(text) else {
                return Ok(USAGE_MSG.into());
            };
            let args = match tokens.as_slice() {
                [] => return Ok(USAGE_MSG.into()),
                [lang] => ThreadCmdArgs::Language {
                    lang: (*lang).to_string(),
                },
                [main, thread] => ThreadCmdArgs::Bilingual {
                    main: (*main).to_string(),
                    thread: (*thread).to_string(),
                },
                _ => return Ok(USAGE_MSG.into()),
            };

            if let Err(msg) = args.validate_languages() {
                return Ok(msg);
            }

            if self.store.in_chat_auto_active(gid) {
                self.store.set_pending_switch(gid, args.to_pending(message));
                return Ok(IN_CHAT_BLOCK_MSG.into());
            }

            return self.handle_thread_subscribe(message, gid, args).await;
        }

        Ok(USAGE_MSG.into())
    }

    async fn handle_thread_subscribe(
        &self,
        message: &BotMessage,
        main_id: &str,
        args: ThreadCmdArgs,
    ) -> AppResult<String> {
        match args {
            ThreadCmdArgs::Language { lang } => {
                self.subscribe_one_arg(message, main_id, &lang).await
            }
            ThreadCmdArgs::Bilingual { main, thread } => {
                Self::subscribe_user_to_bilingual(
                    &self.store,
                    &self.signal,
                    message,
                    main_id,
                    &main,
                    &thread,
                    None,
                    None,
                )
                .await
            }
        }
    }

    async fn subscribe_one_arg(
        &self,
        message: &BotMessage,
        main_id: &str,
        lang_token: &str,
    ) -> AppResult<String> {
        if let Some(bridge) = self.store.get_bridge(main_id) {
            if bridge.is_bilingual() {
                let code = resolved_code(lang_token);
                if bridge.bilingual_thread_lang() == Some(code.as_str()) {
                    return Self::subscribe_user_to_thread(
                        &self.store,
                        &self.signal,
                        message,
                        main_id,
                        lang_token,
                        None,
                        None,
                        None,
                    )
                    .await;
                }
                if bridge.main_lang.as_deref() == Some(code.as_str()) {
                    return Ok(bilingual_main_lang_confirm(&bridge));
                }
                return Ok(bilingual_third_lang_refuse(&bridge));
            }
        }
        Self::subscribe_user_to_thread(
            &self.store,
            &self.signal,
            message,
            main_id,
            lang_token,
            None,
            None,
            None,
        )
        .await
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
                    "Language Threads or Bilingual Threads was not active in this chat. See !translation-threads."
                        .into(),
                );
            }
            return Ok(self
                .apply_pending_in_chat(
                    gid,
                    pending,
                    "Language Threads or Bilingual Threads was not active.",
                )
                .await);
        };

        let bilingual = bridge.is_bilingual();
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
        let prefix = if bilingual {
            "Bilingual Threads disabled."
        } else {
            "Language Threads disabled."
        };
        Ok(self.apply_pending_in_chat(gid, pending, prefix).await)
    }

    async fn notify_sidecars_threads_disabled(&self, bot: &str, bridge: &LanguageBridge) {
        let msg = if bridge.is_bilingual() {
            BILINGUAL_DISABLED_SIDECAR_MSG
        } else {
            THREADS_DISABLED_SIDECAR_MSG
        };
        let mut notified = std::collections::HashSet::new();
        for send_id in bridge.sidecars.values() {
            if !notified.insert(send_id.as_str()) {
                continue;
            }
            if let Err(e) = self.signal.send(bot, send_id, msg).await {
                warn!(
                    error = %e,
                    send_id,
                    "Failed to notify sidecar that threads were disabled"
                );
            } else {
                info!(send_id, "Notified sidecar that threads were disabled");
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

    /// Relay spoken transcript text as if the original speaker posted it.
    pub(crate) async fn fan_out_transcript(&self, original: &BotMessage, spoken: &str) {
        if spoken.trim().is_empty() {
            return;
        }
        let mut msg = original.clone();
        msg.text = spoken.to_string();
        if let Err(e) = self.handle_relay(&msg).await {
            warn!(error = %e, "Language Threads fan-out after transcript failed");
        }
    }

    /// Subscribe to Bilingual Threads (main lang + one sidecar).
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn subscribe_user_to_bilingual(
        store: &Arc<GroupPreferencesStore>,
        signal: &Arc<SignalClient>,
        message: &BotMessage,
        main_id: &str,
        main_lang_token: &str,
        thread_lang_token: &str,
        user_override: Option<&str>,
        address_override: Option<&str>,
    ) -> AppResult<String> {
        let Some(main_lang) = resolve_language(main_lang_token) else {
            return Ok(unknown_lang_msg(main_lang_token));
        };
        let Some(thread_lang) = resolve_language(thread_lang_token) else {
            return Ok(unknown_lang_msg(thread_lang_token));
        };
        if main_lang.code == thread_lang.code {
            return Ok(SAME_LANG_MSG.into());
        }

        if let Some(bridge) = store.get_bridge(main_id) {
            if bridge.is_bilingual() {
                let same_pair = bridge.main_lang.as_deref() == Some(main_lang.code)
                    && bridge.bilingual_thread_lang() == Some(thread_lang.code);
                if !same_pair {
                    return Ok(bilingual_different_pair_refuse(&bridge));
                }
            } else {
                return Ok(LANGUAGE_THREADS_TWO_ARG_REFUSE.into());
            }
        }

        Self::subscribe_user_to_thread(
            store,
            signal,
            message,
            main_id,
            thread_lang_token,
            user_override,
            address_override,
            Some(main_lang.code),
        )
        .await
    }

    /// Subscribe `user` (defaults to message.source) to a language sidecar on `main_id`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn subscribe_user_to_thread(
        store: &Arc<GroupPreferencesStore>,
        signal: &Arc<SignalClient>,
        message: &BotMessage,
        main_id: &str,
        lang_token: &str,
        user_override: Option<&str>,
        address_override: Option<&str>,
        bilingual_main: Option<&str>,
    ) -> AppResult<String> {
        let Some(lang) = resolve_language(lang_token) else {
            return Ok(unknown_lang_msg(lang_token));
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

        if let Some(bridge) = store.get_bridge(main_id) {
            if bridge.is_bilingual() {
                if let Some(thread) = bridge.bilingual_thread_lang() {
                    if lang.code != thread {
                        return Ok(bilingual_third_lang_refuse(&bridge));
                    }
                }
            } else if bilingual_main.is_some() {
                return Ok(LANGUAGE_THREADS_TWO_ARG_REFUSE.into());
            }
        }

        if let Some(existing) = store.member_lang(main_id, &user_key) {
            if existing == lang.code {
                if let Some(main) = bilingual_main {
                    store.set_bilingual_main_lang(main_id, main);
                }
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
            if let Some(main) = bilingual_main {
                store.set_bilingual_main_lang(main_id, main);
            }
        } else {
            let (name, description, welcome) =
                sidecar_copy(lang, message.group_name.as_deref(), main_id);
            match signal
                .create_group(bot, &name, vec![address.clone()], Some(&description))
                .await
            {
                Ok(group) => {
                    if let Some(main) = bilingual_main {
                        store.set_bilingual_sidecar(
                            main_id,
                            main,
                            lang.code,
                            group.id.clone(),
                            group.internal_id.clone(),
                        );
                    } else {
                        store.set_sidecar(
                            main_id,
                            lang.code,
                            group.id.clone(),
                            group.internal_id.clone(),
                        );
                    }
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

        let main_body = if let Some(main_lang_code) = bridge.main_lang.as_deref() {
            let detected = detect_text_language(&spoken);
            if detected.as_deref() == Some(main_lang_code) {
                spoken.clone()
            } else if let Some(target) = resolve_language(main_lang_code) {
                match near_ai_translate(&self.near_ai, &spoken, target).await {
                    Ok(t) => t,
                    Err(e) => {
                        warn!(error = %e, target = %main_lang_code, "Sidecar→main bilingual translate failed");
                        return Ok(());
                    }
                }
            } else {
                warn!(lang = main_lang_code, "Unknown bilingual main language");
                return Ok(());
            }
        } else {
            spoken.clone()
        };
        let to_main = format_attribution(&display, &main_body);

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

        if bridge.is_bilingual() {
            return Ok(());
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

#[async_trait]
impl CommandHandler for TranslateMeHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        if self.bot_identity.is_bot_message(message) {
            return false;
        }
        if message.is_voice_note() {
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
        assert!(TranslateMeHandler::is_on_command(
            "!translate-me-threads es"
        ));
        assert!(TranslateMeHandler::is_on_command(
            "!translation-me-threads es en"
        ));
        assert!(TranslateMeHandler::is_off_command("!leave"));
        assert!(TranslateMeHandler::is_off_command("!LEAVE"));
        assert!(TranslateMeHandler::is_enable_in_chat("!enable-inchat"));
        assert!(TranslateMeHandler::is_enable_in_chat("!enable in-chat"));
        assert!(!TranslateMeHandler::is_command("!translate-on es en"));
        assert!(!TranslateMeHandler::is_command("!translate-me-on es en"));
        assert!(!TranslateMeHandler::is_command("!translate es"));
    }

    #[test]
    fn parses_lang_arg() {
        assert_eq!(
            TranslateMeHandler::thread_tokens("!translate-me-thread es"),
            Some(vec!["es"])
        );
        assert_eq!(
            TranslateMeHandler::thread_tokens("!translate-me-thread es en"),
            Some(vec!["es", "en"])
        );
        assert_eq!(
            TranslateMeHandler::thread_tokens("!translation-me-thread es en"),
            Some(vec!["es", "en"])
        );
        assert_eq!(
            TranslateMeHandler::thread_tokens("!translate-me-threads es"),
            Some(vec!["es"])
        );
        assert_eq!(
            TranslateMeHandler::thread_tokens("!translation-me-threads es en"),
            Some(vec!["es", "en"])
        );
        assert_eq!(
            TranslateMeHandler::thread_tokens("!translate-me-thread"),
            Some(vec![])
        );
        assert_eq!(
            TranslateMeHandler::thread_tokens("!translate-me-thread es en fr"),
            Some(vec!["es", "en", "fr"])
        );
        assert!(ThreadCmdArgs::Bilingual {
            main: "es".into(),
            thread: "es".into(),
        }
        .validate_languages()
        .unwrap_err()
        .contains("two different languages"));
        assert!(ThreadCmdArgs::Language { lang: "zz".into() }
            .validate_languages()
            .unwrap_err()
            .contains("!list-langs"));
        assert!(ThreadCmdArgs::Bilingual {
            main: "es".into(),
            thread: "zz".into(),
        }
        .validate_languages()
        .unwrap_err()
        .contains("!list-langs"));
        assert!(ThreadCmdArgs::Language { lang: "es".into() }
            .validate_languages()
            .is_ok());
        assert!(ThreadCmdArgs::Bilingual {
            main: "es".into(),
            thread: "en".into(),
        }
        .validate_languages()
        .is_ok());
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
                },
                {
                    "name": "EN",
                    "id": "group.en",
                    "internal_id": "en-internal"
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

    async fn send_pairs(signal: &wiremock::MockServer) -> Vec<(String, String)> {
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
            let msg = body["message"].as_str().unwrap_or("").to_string();
            if let Some(arr) = body["recipients"].as_array() {
                for r in arr {
                    if let Some(s) = r.as_str() {
                        out.push((s.to_string(), msg.clone()));
                    }
                }
            }
        }
        out
    }

    async fn create_group_posts(signal: &wiremock::MockServer) -> usize {
        let Some(requests) = signal.received_requests().await else {
            return 0;
        };
        requests
            .iter()
            .filter(|req| {
                let path = req.url.path();
                path.starts_with("/v1/groups/")
                    && !path.contains("/members")
                    && req.method.to_string() == "POST"
            })
            .count()
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

    #[tokio::test]
    async fn fan_out_transcript_skips_empty_spoken() {
        let store = GroupPreferencesStore::new_in_memory(0);
        let handler = handler_pair(
            store,
            "http://127.0.0.1:9".into(),
            "http://127.0.0.1:9".into(),
        );
        handler
            .fan_out_transcript(&group_msg("+alice", ""), "  ")
            .await;
    }

    #[tokio::test]
    async fn fan_out_transcript_relays_non_empty_spoken() {
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
        let handler = handler_pair(store, signal.uri(), near.uri());
        let original = group_msg("+alice", "");
        handler
            .fan_out_transcript(&original, "Hello friends from the mutual aid group")
            .await;

        assert!(
            send_recipients(&signal)
                .await
                .contains(&"group.es".to_string()),
            "Language Threads should send the spoken transcript to the sidecar"
        );
        let messages = send_messages(&signal).await;
        assert!(
            messages.iter().any(|m| m.starts_with("Maria:\n")),
            "sidecar attribution should use the original speaker display name: {messages:?}"
        );
    }

    #[tokio::test]
    async fn bilingual_subscribe_lock_join_and_refuse() {
        use serde_json::json;
        use wiremock::matchers::{method, path, path_regex};
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
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "group.en"})))
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
                    "name": "English · Stacked",
                    "id": "group.en",
                    "internal_id": "en-internal"
                }
            ])))
            .mount(&signal)
            .await;
        Mock::given(method("POST"))
            .and(path_regex(r"^/v1/groups/%2B15550001111/.+/members$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&signal)
            .await;

        let store = GroupPreferencesStore::new_in_memory(0);
        let handler = handler_pair(store.clone(), signal.uri(), near.uri());

        let mut msg = group_msg("+15550002222", "!translate-me-thread es en");
        msg.group_id = Some("main-internal".into());
        assert!(handler.execute(&msg).await.unwrap().is_empty());
        assert!(store.is_bilingual("main-internal"));
        assert_eq!(
            store
                .get_bridge("main-internal")
                .unwrap()
                .main_lang
                .as_deref(),
            Some("es")
        );
        assert_eq!(
            store.lookup_sidecar("en-internal"),
            Some(("main-internal".into(), "en".into()))
        );
        assert_eq!(create_group_posts(&signal).await, 1);

        let created = create_group_posts(&signal).await;

        // Same pair again invites; no second group.
        assert!(handler.execute(&msg).await.unwrap().is_empty());
        assert_eq!(create_group_posts(&signal).await, created);

        // Thread-lang join helper.
        msg.text = "!translate-me-thread en".into();
        let mut other = group_msg("+15550003333", "!translate-me-thread en");
        other.group_id = Some("main-internal".into());
        assert!(handler.execute(&other).await.unwrap().is_empty());
        assert_eq!(
            store
                .member_lang("main-internal", "+15550003333")
                .as_deref(),
            Some("en")
        );
        assert_eq!(create_group_posts(&signal).await, created);

        // Main-lang one-arg does not create a sidecar.
        msg.text = "!translate-me-thread es".into();
        assert!(handler.execute(&msg).await.unwrap().is_empty());
        let bodies = send_messages(&signal).await;
        assert!(
            bodies.iter().any(|m| m.contains("main chat is Spanish")),
            "{bodies:?}"
        );
        assert_eq!(create_group_posts(&signal).await, created);

        // Third lang / swapped pair refused.
        msg.text = "!translate-me-thread fr".into();
        assert!(handler.execute(&msg).await.unwrap().is_empty());
        msg.text = "!translate-me-thread en es".into();
        assert!(handler.execute(&msg).await.unwrap().is_empty());
        let bodies = send_messages(&signal).await;
        assert!(
            bodies
                .iter()
                .any(|m| m.contains("third language isn't supported")),
            "{bodies:?}"
        );
        assert!(
            bodies
                .iter()
                .any(|m| m.contains("different pair isn't supported")),
            "{bodies:?}"
        );
        assert_eq!(create_group_posts(&signal).await, created);
        assert_eq!(store.get_bridge("main-internal").unwrap().sidecars.len(), 1);
    }

    #[tokio::test]
    async fn language_threads_refuses_two_arg_after_lock() {
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
                    "name": "Main",
                    "id": "group.main",
                    "internal_id": "main-internal"
                },
                {
                    "name": "Spanish",
                    "id": "group.es",
                    "internal_id": "es-internal"
                }
            ])))
            .mount(&signal)
            .await;

        let store = GroupPreferencesStore::new_in_memory(0);
        let handler = handler_pair(store.clone(), signal.uri(), "http://127.0.0.1:9".into());
        let mut msg = group_msg("+15550002222", "!translate-me-thread es");
        assert!(handler.execute(&msg).await.unwrap().is_empty());
        assert!(store.is_language_threads("main-internal"));
        assert!(!store.is_bilingual("main-internal"));

        msg.text = "!translate-me-thread es en".into();
        assert!(handler.execute(&msg).await.unwrap().is_empty());
        let bodies = send_messages(&signal).await;
        assert!(
            bodies
                .iter()
                .any(|m| m.contains("Language Threads is already on")),
            "{bodies:?}"
        );
        assert!(!store.is_bilingual("main-internal"));
    }

    #[tokio::test]
    async fn in_chat_blocks_both_thread_forms_and_stashes_pending() {
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
            }])))
            .mount(&signal)
            .await;

        let store = GroupPreferencesStore::new_in_memory(0);
        let mode = GroupTranslateMode::new(
            resolve_language("es").unwrap(),
            resolve_language("en").unwrap(),
        );
        store.set("main-internal".into(), mode);
        let handler = handler_pair(store.clone(), signal.uri(), "http://127.0.0.1:9".into());

        let mut one = group_msg("+15550002222", "!translate-me-thread es");
        one.group_id = Some("main-internal".into());
        assert!(handler.execute(&one).await.unwrap().is_empty());
        assert!(matches!(
            store.get_pending_switch("main-internal"),
            Some(PendingSwitch::EnableThreads { .. })
        ));

        let mut two = group_msg("+15550002222", "!translate-me-thread es en");
        two.group_id = Some("main-internal".into());
        assert!(handler.execute(&two).await.unwrap().is_empty());
        assert!(matches!(
            store.get_pending_switch("main-internal"),
            Some(PendingSwitch::EnableBilingualThreads { .. })
        ));
        let bodies = send_messages(&signal).await;
        assert!(
            bodies
                .iter()
                .any(|m| m.contains("Language Threads and Bilingual Threads")),
            "{bodies:?}"
        );
        assert!(!store.threads_active("main-internal"));
    }

    #[tokio::test]
    async fn enable_threads_applies_bilingual_pending() {
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
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "group.en"})))
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
                    "name": "English",
                    "id": "group.en",
                    "internal_id": "en-internal"
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

        let mut blocked = group_msg("+15550002222", "!translate-me-thread es en");
        blocked.group_id = Some("main-internal".into());
        assert!(translate_me.execute(&blocked).await.unwrap().is_empty());
        assert!(matches!(
            store.get_pending_switch("main-internal"),
            Some(PendingSwitch::EnableBilingualThreads { .. })
        ));

        let mut enable = group_msg("+15550002222", "!enable-threads");
        enable.group_id = Some("main-internal".into());
        assert!(translate_all.execute(&enable).await.unwrap().is_empty());
        assert!(store.is_bilingual("main-internal"));
        assert_eq!(
            store
                .get_bridge("main-internal")
                .unwrap()
                .main_lang
                .as_deref(),
            Some("es")
        );
        assert_eq!(
            store.lookup_sidecar("en-internal"),
            Some(("main-internal".into(), "en".into()))
        );
    }

    #[tokio::test]
    async fn bilingual_relay_both_directions() {
        let signal = wiremock::MockServer::start().await;
        let near = wiremock::MockServer::start().await;
        mount_relay_signal(&signal).await;
        mount_near(&near).await;

        let store = GroupPreferencesStore::new_in_memory(0);
        store.set_bilingual_sidecar(
            "main-internal",
            "es",
            "en",
            "group.en".into(),
            "en-internal".into(),
        );
        let handler = handler_pair(store, signal.uri(), near.uri());

        // Main English → thread English: relay (already thread lang).
        let main_en = group_msg(
            "+15550002222",
            "Hello friends from the mutual aid group tonight",
        );
        assert!(handler.execute(&main_en).await.unwrap().is_empty());
        let pairs = send_pairs(&signal).await;
        let to_en: Vec<_> = pairs
            .iter()
            .filter(|(r, _)| r == "group.en")
            .map(|(_, m)| m.as_str())
            .collect();
        assert!(
            to_en.iter().any(|m| m.contains("Hello friends")),
            "{to_en:?}"
        );

        // Main Spanish → thread: translate.
        let main_es = group_msg(
            "+15550002222",
            "Hola amigos del grupo de ayuda mutua esta noche",
        );
        assert!(handler.execute(&main_es).await.unwrap().is_empty());
        let pairs = send_pairs(&signal).await;
        assert!(
            pairs
                .iter()
                .any(|(r, m)| r == "group.en" && m.contains("Translated")),
            "{pairs:?}"
        );

        // Thread English → main Spanish: translate.
        let mut side_en = group_msg(
            "+15550002222",
            "Hello everyone in the sidecar mutual aid chat",
        );
        side_en.group_id = Some("en-internal".into());
        assert!(handler.execute(&side_en).await.unwrap().is_empty());
        let pairs = send_pairs(&signal).await;
        assert!(
            pairs
                .iter()
                .any(|(r, m)| r == "group.main" && m.contains("Translated")),
            "{pairs:?}"
        );
        assert!(
            !pairs
                .iter()
                .any(|(r, _)| r == "group.es" || r == "group.fr"),
            "bilingual must not fan out to extra sidecars: {pairs:?}"
        );

        // Thread already-Spanish → main: relay original.
        let mut side_es = group_msg(
            "+15550002222",
            "Hola, ¿cómo estás ustedes? Hoy es miércoles y tengo tres bananas.",
        );
        side_es.group_id = Some("en-internal".into());
        assert!(handler.execute(&side_es).await.unwrap().is_empty());
        let pairs = send_pairs(&signal).await;
        assert!(
            pairs.iter().any(|(r, m)| r == "group.main"
                && m.contains("tres bananas")
                && !m.contains("Translated")),
            "{pairs:?}"
        );
    }

    #[tokio::test]
    async fn language_threads_n1_sidecar_to_main_stays_untranslated() {
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
        let handler = handler_pair(store, signal.uri(), near.uri());
        let mut side = group_msg("+15550002222", "Hola desde el thread de ayuda mutua");
        side.group_id = Some("es-internal".into());
        assert!(handler.execute(&side).await.unwrap().is_empty());
        let pairs = send_pairs(&signal).await;
        assert!(
            pairs.iter().any(|(r, m)| r == "group.main"
                && m.contains("Hola desde")
                && !m.contains("Translated")),
            "{pairs:?}"
        );
    }

    #[tokio::test]
    async fn fan_out_transcript_uses_bilingual_sidecar_to_main() {
        let signal = wiremock::MockServer::start().await;
        let near = wiremock::MockServer::start().await;
        mount_relay_signal(&signal).await;
        mount_near(&near).await;

        let store = GroupPreferencesStore::new_in_memory(0);
        store.set_bilingual_sidecar(
            "main-internal",
            "es",
            "en",
            "group.en".into(),
            "en-internal".into(),
        );
        let handler = handler_pair(store, signal.uri(), near.uri());
        let mut original = group_msg("+alice", "");
        original.group_id = Some("en-internal".into());
        handler
            .fan_out_transcript(&original, "Hello friends from the mutual aid group tonight")
            .await;

        let pairs = send_pairs(&signal).await;
        assert!(
            pairs.iter().any(|(r, m)| r == "group.main"
                && m.starts_with("Maria:\n")
                && m.contains("Translated")),
            "{pairs:?}"
        );
    }

    #[tokio::test]
    async fn same_lang_pair_and_extra_tokens_refused() {
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
            }])))
            .mount(&signal)
            .await;

        let store = GroupPreferencesStore::new_in_memory(0);
        let handler = handler_pair(store, signal.uri(), "http://127.0.0.1:9".into());

        let mut same = group_msg("+15550002222", "!translate-me-thread es es");
        same.group_id = Some("main-internal".into());
        assert!(handler.execute(&same).await.unwrap().is_empty());

        let mut extra = group_msg("+15550002222", "!translate-me-thread es en fr");
        extra.group_id = Some("main-internal".into());
        assert!(handler.execute(&extra).await.unwrap().is_empty());

        let bodies = send_messages(&signal).await;
        assert!(
            bodies.iter().any(|m| m.contains("two different languages")),
            "{bodies:?}"
        );
        assert!(bodies.iter().any(|m| m.contains("Usage:")), "{bodies:?}");
    }
}
