//! Parallel Translation: monolingual main + one parallel Signal group, bidirectional relay.

use crate::bot_identity::BotIdentity;
use crate::commands::translate_lang::resolve_language;
use crate::commands::translate_service::{detect_text_language, near_ai_translate};
use crate::commands::CommandHandler;
use crate::error::AppResult;
use crate::group_preferences_store::{GroupPreferencesStore, ParallelBridge};
use async_trait::async_trait;
use near_ai_client::NearAiClient;
use signal_client::{BotMessage, SignalClient};
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

const GROUP_ONLY: &str = "Parallel commands are only available in group chats.";
const USAGE: &str = "Usage: !parallel-on <lang_this_chat> <lang_parallel>\n\
Example: !parallel-on en es\n\
lang1 = THIS chat; the bot creates a parallel group for lang2.\n\
Then each person runs !parallel-join to be added.";

pub struct TranslateParallelHandler {
    store: Arc<GroupPreferencesStore>,
    near_ai: Arc<NearAiClient>,
    signal: Arc<SignalClient>,
    bot_identity: Arc<BotIdentity>,
}

impl TranslateParallelHandler {
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

    fn starts_with_word(text: &str, prefix: &str) -> bool {
        text.strip_prefix(prefix)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\n'))
    }

    fn is_on(text: &str) -> bool {
        Self::starts_with_word(text, "!parallel-on")
    }

    fn is_off(text: &str) -> bool {
        text == "!parallel-off"
    }

    fn is_join(text: &str) -> bool {
        text == "!parallel-join"
    }

    fn is_leave(text: &str) -> bool {
        text == "!parallel-leave"
    }

    fn parse_lang_pair(text: &str) -> Option<(&str, &str)> {
        let rest = text.trim().strip_prefix("!parallel-on")?.trim();
        let mut parts = rest.split_whitespace();
        let a = parts.next()?;
        let b = parts.next()?;
        if parts.next().is_some() {
            return None;
        }
        Some((a, b))
    }

    fn is_relay_candidate(&self, message: &BotMessage) -> bool {
        let text = message.text.trim();
        if message.group_id.is_none()
            || message.is_voice_note()
            || text.is_empty()
            || text.starts_with('!')
        {
            return false;
        }
        let Some(gid) = message.group_id.as_deref() else {
            return false;
        };
        self.store.get_parallel(gid).is_some() || self.store.lookup_parallel(gid).is_some()
    }

    async fn handle_on(&self, message: &BotMessage) -> AppResult<String> {
        let Some(main_id) = message.group_id.as_deref() else {
            return Ok(GROUP_ONLY.into());
        };

        if self.store.lookup_parallel(main_id).is_some() {
            return Ok("Run !parallel-on in the main group (not inside the parallel chat).".into());
        }

        if self.store.is_active(main_id) {
            return Ok(
                "In-chat auto-translate is active here. Run !translate-off before enabling Parallel."
                    .into(),
            );
        }

        if self.store.has_parallel(main_id) {
            let bridge = self.store.get_parallel(main_id).unwrap();
            return Ok(format!(
                "Parallel is already on: this chat = {}, parallel = {}. \
Use !parallel-join to join the parallel group, or !parallel-off to stop.",
                bridge.main_lang, bridge.parallel_lang
            ));
        }

        let Some((token_a, token_b)) = Self::parse_lang_pair(message.text.trim()) else {
            return Ok(USAGE.into());
        };

        let Some(lang_main) = resolve_language(token_a) else {
            return Ok(format!(
                "Unknown language `{token_a}`. Try !list-langs for supported codes."
            ));
        };
        let Some(lang_parallel) = resolve_language(token_b) else {
            return Ok(format!(
                "Unknown language `{token_b}`. Try !list-langs for supported codes."
            ));
        };

        if lang_main.code == lang_parallel.code {
            return Ok("Choose two different languages. Example: !parallel-on en es".into());
        }

        let Some(address) = message.invite_address() else {
            return Ok(
                "Could not determine your Signal address. Message the bot in a DM once, then retry."
                    .into(),
            );
        };

        let bot = &message.receiving_account;
        let name = format!("Parallel {}", lang_parallel.name);
        let description = format!(
            "Parallel {} lane bridged to the main group ({}).",
            lang_parallel.name, lang_main.name
        );

        let group = match self
            .signal
            .create_group(bot, &name, vec![address.clone()], Some(&description))
            .await
        {
            Ok(g) => g,
            Err(e) => {
                return Ok(format!(
                    "Could not create the parallel group: {e}. Try again shortly."
                ));
            }
        };

        let bridge = ParallelBridge {
            main_lang: lang_main.code.to_string(),
            parallel_lang: lang_parallel.code.to_string(),
            parallel_send_id: group.id.clone(),
            parallel_internal_id: group.internal_id.clone(),
            members: [(message.source.clone(), address)].into_iter().collect(),
        };
        self.store.set_parallel(main_id, bridge);

        let welcome = format!(
            "Welcome to Parallel {}. Messages here are translated to/from the main group ({}).",
            lang_parallel.name, lang_main.name
        );
        if let Err(e) = self.signal.send(bot, &group.id, &welcome).await {
            warn!(error = %e, "Failed to send parallel welcome");
        }

        info!(
            main_id,
            main_lang = lang_main.code,
            parallel_lang = lang_parallel.code,
            "parallel-on: created parallel bridge"
        );

        Ok(format!(
            "Parallel enabled: this chat = {} {}, parallel = {} {}.\n\
Accept the Signal invite to \"{}\" if prompted.\n\
Others in this group: run !parallel-join to be added to the parallel chat.\n\
!parallel-off stops Parallel for the group.",
            lang_main.flag, lang_main.name, lang_parallel.flag, lang_parallel.name, name
        ))
    }

    async fn handle_join(&self, message: &BotMessage) -> AppResult<String> {
        let Some(gid) = message.group_id.as_deref() else {
            return Ok(GROUP_ONLY.into());
        };

        let main_id = if self.store.lookup_parallel(gid).is_some() {
            return Ok(
                "You are already in the parallel chat. Use !parallel-leave to leave, \
or run !parallel-join from the main group to re-invite."
                    .into(),
            );
        } else if self.store.has_parallel(gid) {
            gid.to_string()
        } else {
            return Ok(
                "Parallel is not set up here. An organizer should run !parallel-on <lang1> <lang2> first."
                    .into(),
            );
        };

        let Some(bridge) = self.store.get_parallel(&main_id) else {
            return Ok("Parallel is not active.".into());
        };

        if bridge.is_member(&message.source) {
            return Ok(
                "You are already on the parallel group roster. Accept the Signal invite if pending."
                    .into(),
            );
        }

        let Some(address) = message.invite_address() else {
            return Ok(
                "Could not determine your Signal address. Message the bot in a DM once, then retry."
                    .into(),
            );
        };

        if let Err(e) = self
            .signal
            .add_members(
                &message.receiving_account,
                &bridge.parallel_send_id,
                vec![address.clone()],
            )
            .await
        {
            return Ok(format!("Could not add you to the parallel group: {e}"));
        }

        self.store
            .add_parallel_member(&main_id, &message.source, address);

        let lang_name = resolve_language(&bridge.parallel_lang)
            .map(|l| l.name)
            .unwrap_or(bridge.parallel_lang.as_str());
        Ok(format!(
            "Joined Parallel {lang_name}. Accept the Signal group invite if prompted."
        ))
    }

    async fn handle_leave(&self, message: &BotMessage) -> AppResult<String> {
        let Some(gid) = message.group_id.as_deref() else {
            return Ok(GROUP_ONLY.into());
        };

        let main_id = if let Some(main) = self.store.lookup_parallel(gid) {
            main
        } else if self.store.has_parallel(gid) {
            gid.to_string()
        } else {
            return Ok("Parallel is not active in this chat.".into());
        };

        let Some(bridge) = self.store.get_parallel(&main_id) else {
            return Ok("Parallel is not active.".into());
        };

        let address = self
            .store
            .remove_parallel_member(&main_id, &message.source)
            .or_else(|| message.invite_address())
            .unwrap_or_else(|| message.source.clone());

        if let Err(e) = self
            .signal
            .remove_members(
                &message.receiving_account,
                &bridge.parallel_send_id,
                vec![address],
            )
            .await
        {
            warn!(error = %e, "Failed to remove member from parallel group");
        }

        Ok("Left the parallel group.".into())
    }

    async fn handle_off(&self, message: &BotMessage) -> AppResult<String> {
        let Some(gid) = message.group_id.as_deref() else {
            return Ok(GROUP_ONLY.into());
        };

        if self.store.lookup_parallel(gid).is_some() {
            return Ok("Run !parallel-off in the main group to stop Parallel.".into());
        }

        if self.store.clear_parallel(gid) {
            info!(main_id = gid, "parallel-off: cleared bridge");
            Ok("Parallel disabled for this group. Existing parallel Signal group is left as-is; you can leave it manually.".into())
        } else {
            Ok("Parallel was not active in this chat.".into())
        }
    }

    #[instrument(skip(self, message))]
    async fn handle_relay(&self, message: &BotMessage) -> AppResult<()> {
        if self.bot_identity.is_bot_message(message) {
            debug!("Skipping bot-authored message for parallel relay");
            return Ok(());
        }

        let Some(gid) = message.group_id.as_deref() else {
            return Ok(());
        };

        if let Some(main_id) = self.store.lookup_parallel(gid) {
            if !self.store.allow_message(&main_id) {
                warn!(main_id, "Rate limit: skipping parallel→main");
                return Ok(());
            }
            return self.relay_parallel_to_main(message, &main_id).await;
        }

        if let Some(bridge) = self.store.get_parallel(gid) {
            if !self.store.allow_message(gid) {
                warn!(main_id = gid, "Rate limit: skipping main→parallel");
                return Ok(());
            }
            return self.relay_main_to_parallel(message, &bridge).await;
        }

        Ok(())
    }

    async fn relay_main_to_parallel(
        &self,
        message: &BotMessage,
        bridge: &ParallelBridge,
    ) -> AppResult<()> {
        let Some(target) = resolve_language(&bridge.parallel_lang) else {
            return Ok(());
        };
        let detected = detect_text_language(&message.text);
        let body = if detected.as_deref() == Some(bridge.parallel_lang.as_str()) {
            message.text.clone()
        } else {
            match near_ai_translate(&self.near_ai, &message.text, target).await {
                Ok(t) => t,
                Err(e) => {
                    warn!("parallel main→lane translation failed: {e}");
                    return Ok(());
                }
            }
        };
        let attributed = format_attribution(&message.display_name(), &body);
        if let Err(e) = self
            .signal
            .send(
                &message.receiving_account,
                &bridge.parallel_send_id,
                &attributed,
            )
            .await
        {
            warn!(error = %e, "Failed to post to parallel group");
        }
        Ok(())
    }

    async fn relay_parallel_to_main(&self, message: &BotMessage, main_id: &str) -> AppResult<()> {
        let Some(bridge) = self.store.get_parallel(main_id) else {
            return Ok(());
        };
        let Some(target) = resolve_language(&bridge.main_lang) else {
            return Ok(());
        };
        let detected = detect_text_language(&message.text);
        let body = if detected.as_deref() == Some(bridge.main_lang.as_str()) {
            message.text.clone()
        } else {
            match near_ai_translate(&self.near_ai, &message.text, target).await {
                Ok(t) => t,
                Err(e) => {
                    warn!("parallel lane→main translation failed: {e}");
                    return Ok(());
                }
            }
        };
        let attributed = format_attribution(&message.display_name(), &body);
        let bot = &message.receiving_account;
        let main_recipient = match self
            .signal
            .resolve_group_send_id_for_account(bot, main_id)
            .await
        {
            Ok(id) => id,
            Err(e) => {
                warn!(error = %e, main_id, "Could not resolve main send id for parallel relay");
                return Ok(());
            }
        };
        if let Err(e) = self.signal.send(bot, &main_recipient, &attributed).await {
            warn!(error = %e, "Failed to post to main group from parallel");
        }
        Ok(())
    }

    async fn handle_command(&self, message: &BotMessage) -> AppResult<String> {
        let text = message.text.trim();
        if Self::is_off(text) {
            self.handle_off(message).await
        } else if Self::is_join(text) {
            self.handle_join(message).await
        } else if Self::is_leave(text) {
            self.handle_leave(message).await
        } else if Self::is_on(text) {
            self.handle_on(message).await
        } else {
            Ok(USAGE.into())
        }
    }
}

fn format_attribution(display_name: &str, body: &str) -> String {
    format!("{display_name}:\n{body}")
}

#[async_trait]
impl CommandHandler for TranslateParallelHandler {
    fn label(&self) -> &'static str {
        "translate_parallel"
    }

    fn matches(&self, message: &BotMessage) -> bool {
        let text = message.text.trim();
        Self::is_on(text)
            || Self::is_off(text)
            || Self::is_join(text)
            || Self::is_leave(text)
            || self.is_relay_candidate(message)
    }

    fn handles_own_reply(&self) -> bool {
        true
    }

    #[instrument(skip(self, message), fields(source = %message.source))]
    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        let text = message.text.trim();
        if text.starts_with('!') {
            let reply = self.handle_command(message).await?;
            self.signal.reply(message, &reply).await?;
            Ok(String::new())
        } else {
            self.handle_relay(message).await?;
            Ok(String::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::translate_lang::resolve_language;
    use crate::group_preferences_store::GroupTranslateMode;
    use serde_json::json;
    use wiremock::matchers::{method, path, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn group_msg(source: &str, text: &str) -> BotMessage {
        BotMessage {
            source: source.into(),
            source_number: Some(source.into()),
            source_name: Some("Ada".into()),
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

    fn handler_pair(
        store: Arc<GroupPreferencesStore>,
        signal_uri: String,
        near_uri: String,
    ) -> TranslateParallelHandler {
        TranslateParallelHandler::new(
            store,
            Arc::new(
                NearAiClient::new("key", near_uri, "m", std::time::Duration::from_secs(5)).unwrap(),
            ),
            Arc::new(SignalClient::new(signal_uri).unwrap()),
            BotIdentity::new(),
        )
    }

    fn sample_bridge() -> ParallelBridge {
        ParallelBridge {
            main_lang: "en".into(),
            parallel_lang: "es".into(),
            parallel_send_id: "group.es".into(),
            parallel_internal_id: "es-internal".into(),
            members: [("+15550002222".into(), "+15550002222".into())]
                .into_iter()
                .collect(),
        }
    }

    async fn mount_signal_basics(signal: &MockServer) {
        Mock::given(method("POST"))
            .and(path("/v2/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(signal)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/groups/%2B15550001111"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "group.es"})))
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
                    "name": "Parallel Spanish",
                    "id": "group.es",
                    "internal_id": "es-internal"
                }
            ])))
            .mount(signal)
            .await;
        Mock::given(method("POST"))
            .and(path_regex(r"^/v1/groups/%2B15550001111/.+/members$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(signal)
            .await;
        Mock::given(method("DELETE"))
            .and(path_regex(r"^/v1/groups/%2B15550001111/.+/members$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(signal)
            .await;
    }

    #[test]
    fn parse_lang_pair_ok() {
        assert_eq!(
            TranslateParallelHandler::parse_lang_pair("!parallel-on en es"),
            Some(("en", "es"))
        );
        assert!(TranslateParallelHandler::parse_lang_pair("!parallel-on en").is_none());
        assert!(TranslateParallelHandler::parse_lang_pair("!parallel-on en es fr").is_none());
    }

    #[test]
    fn command_matchers() {
        assert!(TranslateParallelHandler::is_on("!parallel-on en es"));
        assert!(!TranslateParallelHandler::is_on("!parallel"));
        assert!(TranslateParallelHandler::is_join("!parallel-join"));
        assert!(TranslateParallelHandler::is_leave("!parallel-leave"));
        assert!(TranslateParallelHandler::is_off("!parallel-off"));
    }

    #[test]
    fn attribution_format() {
        assert_eq!(format_attribution("Ada", "hola"), "Ada:\nhola");
    }

    #[test]
    fn matches_commands_and_relay_candidates() {
        let store = GroupPreferencesStore::new_in_memory(0);
        store.set_parallel("main-internal", sample_bridge());
        let handler = handler_pair(
            store,
            "http://127.0.0.1:9".into(),
            "http://127.0.0.1:9".into(),
        );

        assert!(handler.matches(&group_msg("+1", "!parallel-on en es")));
        assert!(handler.matches(&group_msg("+1", "!parallel-join")));
        assert!(handler.matches(&group_msg("+1", "!parallel-leave")));
        assert!(handler.matches(&group_msg("+1", "!parallel-off")));

        let relay = group_msg("+15550002222", "Hello from the main chat");
        assert!(handler.matches(&relay));

        let mut cmd = relay.clone();
        cmd.text = "!help".into();
        assert!(!handler.matches(&cmd));

        let mut dm = relay.clone();
        dm.is_group = false;
        dm.group_id = None;
        dm.text = "hello".into();
        assert!(!handler.matches(&dm));
    }

    #[tokio::test]
    async fn setup_join_leave_off_happy_path() {
        let signal = MockServer::start().await;
        mount_signal_basics(&signal).await;

        let store = GroupPreferencesStore::new_in_memory(0);
        let handler = handler_pair(store.clone(), signal.uri(), "http://127.0.0.1:9".into());

        let on = group_msg("+15550002222", "!parallel-on en es");
        assert!(handler.execute(&on).await.unwrap().is_empty());
        assert!(store.has_parallel("main-internal"));

        // Already on.
        assert!(handler.execute(&on).await.unwrap().is_empty());

        let join = group_msg("+15550003333", "!parallel-join");
        assert!(handler.execute(&join).await.unwrap().is_empty());
        assert!(store
            .get_parallel("main-internal")
            .unwrap()
            .is_member("+15550003333"));

        // Already a member.
        assert!(handler.execute(&join).await.unwrap().is_empty());

        let leave = group_msg("+15550003333", "!parallel-leave");
        assert!(handler.execute(&leave).await.unwrap().is_empty());

        let off = group_msg("+15550002222", "!parallel-off");
        assert!(handler.execute(&off).await.unwrap().is_empty());
        assert!(!store.has_parallel("main-internal"));
    }

    #[tokio::test]
    async fn setup_rejects_invalid_and_conflicting_states() {
        let signal = MockServer::start().await;
        mount_signal_basics(&signal).await;

        let store = GroupPreferencesStore::new_in_memory(0);
        let handler = handler_pair(store.clone(), signal.uri(), "http://127.0.0.1:9".into());

        let dm = BotMessage {
            source: "+15550002222".into(),
            source_number: Some("+15550002222".into()),
            source_name: None,
            text: "!parallel-on en es".into(),
            timestamp: 1,
            message_timestamp: 1,
            is_group: false,
            group_id: None,
            group_name: None,
            receiving_account: "+15550001111".into(),
            attachments: vec![],
            quote: None,
        };
        assert!(handler.execute(&dm).await.unwrap().is_empty());

        let usage = group_msg("+15550002222", "!parallel-on");
        assert!(handler.execute(&usage).await.unwrap().is_empty());

        let unknown = group_msg("+15550002222", "!parallel-on en zz");
        assert!(handler.execute(&unknown).await.unwrap().is_empty());

        let same = group_msg("+15550002222", "!parallel-on en english");
        assert!(handler.execute(&same).await.unwrap().is_empty());

        store.set(
            "main-internal".into(),
            GroupTranslateMode::new(
                resolve_language("en").unwrap(),
                resolve_language("es").unwrap(),
            ),
        );
        let blocked = group_msg("+15550002222", "!parallel-on en es");
        assert!(handler.execute(&blocked).await.unwrap().is_empty());
        store.clear("main-internal");

        store.set_parallel("main-internal", sample_bridge());
        let mut from_parallel = group_msg("+15550002222", "!parallel-on en fr");
        from_parallel.group_id = Some("es-internal".into());
        assert!(handler.execute(&from_parallel).await.unwrap().is_empty());

        from_parallel.text = "!parallel-join".into();
        assert!(handler.execute(&from_parallel).await.unwrap().is_empty());

        from_parallel.text = "!parallel-off".into();
        assert!(handler.execute(&from_parallel).await.unwrap().is_empty());
        assert!(store.has_parallel("main-internal"));
    }

    #[tokio::test]
    async fn join_without_setup_and_leave_from_parallel() {
        let signal = MockServer::start().await;
        mount_signal_basics(&signal).await;

        let store = GroupPreferencesStore::new_in_memory(0);
        let handler = handler_pair(store.clone(), signal.uri(), "http://127.0.0.1:9".into());

        let join = group_msg("+15550003333", "!parallel-join");
        assert!(handler.execute(&join).await.unwrap().is_empty());

        store.set_parallel("main-internal", sample_bridge());
        let mut leave = group_msg("+15550002222", "!parallel-leave");
        leave.group_id = Some("es-internal".into());
        assert!(handler.execute(&leave).await.unwrap().is_empty());
        assert!(!store
            .get_parallel("main-internal")
            .unwrap()
            .is_member("+15550002222"));
    }

    #[tokio::test]
    async fn relay_main_to_parallel_and_back() {
        let signal = MockServer::start().await;
        let near = MockServer::start().await;
        mount_signal_basics(&signal).await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "1",
                "choices": [{"index": 0, "message": {"role": "assistant", "content": "Hola"}, "finish_reason": "stop"}],
                "created": 1,
                "model": "m",
                "object": "chat.completion"
            })))
            .mount(&near)
            .await;

        let store = GroupPreferencesStore::new_in_memory(0);
        store.set_parallel("main-internal", sample_bridge());
        let handler = handler_pair(store, signal.uri(), near.uri());

        let main_msg = group_msg(
            "+15550002222",
            "Hello friends from the mutual aid meetup tonight",
        );
        assert!(handler.matches(&main_msg));
        assert!(handler.execute(&main_msg).await.unwrap().is_empty());

        let mut side = group_msg(
            "+15550002222",
            "Necesitamos más voluntarios para el evento de mañana",
        );
        side.group_id = Some("es-internal".into());
        assert!(handler.matches(&side));
        assert!(handler.execute(&side).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn relay_skips_bot_authored_messages() {
        let signal = MockServer::start().await;
        mount_signal_basics(&signal).await;

        let store = GroupPreferencesStore::new_in_memory(0);
        store.set_parallel("main-internal", sample_bridge());
        let identity = BotIdentity::new();
        identity.remember_phone("+15550001111");
        let handler = TranslateParallelHandler::new(
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

        let bot_msg = group_msg("+15550001111", "Ada:\nHola");
        assert!(handler.execute(&bot_msg).await.unwrap().is_empty());
    }
}
