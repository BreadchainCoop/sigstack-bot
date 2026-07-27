//! Nested product menus: `!translation`, `!transcription`, `!in-chat`.

use crate::commands::menu_locale::{
    help_menu, in_chat_menu, is_exact_command, menu_language_for_message, transcription_group_only,
    transcription_invited, transcription_unavailable, translation_products_menu,
};
use crate::commands::CommandHandler;
use crate::config::BotRole;
use crate::error::AppResult;
use crate::group_preferences_store::GroupPreferencesStore;
use async_trait::async_trait;
use signal_client::{BotMessage, SignalClient};
use std::sync::Arc;
use tracing::warn;

pub struct TranslationMenuHandler {
    group_prefs: Arc<GroupPreferencesStore>,
}

impl TranslationMenuHandler {
    pub fn new(group_prefs: Arc<GroupPreferencesStore>) -> Self {
        Self { group_prefs }
    }
}

#[async_trait]
impl CommandHandler for TranslationMenuHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        is_exact_command(&message.text, "!translation")
    }

    fn label(&self) -> &'static str {
        "translation_menu"
    }

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        let language = menu_language_for_message(message, &self.group_prefs);
        Ok(translation_products_menu(language).into())
    }
}

/// Translation role: invite the transcription peer, or stay silent when already paired.
pub struct TranscriptionPairingHandler {
    group_prefs: Arc<GroupPreferencesStore>,
    signal: Arc<SignalClient>,
    peer_phone: Option<String>,
}

impl TranscriptionPairingHandler {
    pub fn new(
        group_prefs: Arc<GroupPreferencesStore>,
        signal: Arc<SignalClient>,
        peer_phone: Option<String>,
    ) -> Self {
        Self {
            group_prefs,
            signal,
            peer_phone: peer_phone.and_then(|p| {
                let t = p.trim().to_string();
                if t.is_empty() {
                    None
                } else {
                    Some(t)
                }
            }),
        }
    }

    async fn send(&self, message: &BotMessage, body: &str) -> AppResult<()> {
        self.signal.reply(message, body).await?;
        Ok(())
    }
}

#[async_trait]
impl CommandHandler for TranscriptionPairingHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        is_exact_command(&message.text, "!transcription")
    }

    fn handles_own_reply(&self) -> bool {
        true
    }

    fn label(&self) -> &'static str {
        "transcription_pairing"
    }

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        let language = menu_language_for_message(message, &self.group_prefs);

        if !message.is_group {
            self.send(message, transcription_group_only(language))
                .await?;
            return Ok(String::new());
        }

        let Some(peer) = self.peer_phone.as_deref() else {
            self.send(message, transcription_unavailable(language))
                .await?;
            return Ok(String::new());
        };

        let Some(group_id) = message.group_id.as_deref() else {
            self.send(message, transcription_unavailable(language))
                .await?;
            return Ok(String::new());
        };

        let bot = message.receiving_account.as_str();
        let groups = match self.signal.list_groups(bot).await {
            Ok(g) => g,
            Err(e) => {
                warn!(error = %e, "Failed to list groups for transcription pairing");
                self.send(message, "Could not look up this group. Try again shortly.")
                    .await?;
                return Ok(String::new());
            }
        };

        let Some(group) = groups
            .iter()
            .find(|g| g.internal_id == group_id || g.id == group_id)
        else {
            self.send(message, transcription_unavailable(language))
                .await?;
            return Ok(String::new());
        };

        if group.contains_member_or_pending(peer) {
            // Paired (or invite pending): stay silent so the transcription bot can answer.
            return Ok(String::new());
        }

        let send_id = match self
            .signal
            .resolve_group_send_id_for_account(bot, group_id)
            .await
        {
            Ok(id) => id,
            Err(e) => {
                warn!(error = %e, "Failed to resolve group send id for pairing");
                self.send(
                    message,
                    "Could not resolve this group for invites. Try again shortly.",
                )
                .await?;
                return Ok(String::new());
            }
        };

        match self
            .signal
            .add_members(bot, &send_id, vec![peer.to_string()])
            .await
        {
            Ok(()) => {
                self.send(message, transcription_invited(language)).await?;
            }
            Err(e) => {
                warn!(error = %e, peer, "Failed to invite transcription bot");
                let body = format!(
                    "Could not add the transcription bot ({peer}): {e}\n\n\
                     This bot must be a group admin to invite members. \
                     Or set SIGNAL__PEER_PHONE and try again.\n\n\
                     !help — Main menu"
                );
                self.send(message, &body).await?;
            }
        }

        Ok(String::new())
    }
}

/// Transcription role: product menu for `!transcription`.
pub struct TranscriptionMenuHandler {
    group_prefs: Arc<GroupPreferencesStore>,
}

impl TranscriptionMenuHandler {
    pub fn new(group_prefs: Arc<GroupPreferencesStore>) -> Self {
        Self { group_prefs }
    }
}

#[async_trait]
impl CommandHandler for TranscriptionMenuHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        is_exact_command(&message.text, "!transcription")
    }

    fn label(&self) -> &'static str {
        "transcription_menu"
    }

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        let language = menu_language_for_message(message, &self.group_prefs);
        Ok(help_menu(language, BotRole::Transcription).into())
    }
}

pub struct InChatMenuHandler {
    group_prefs: Arc<GroupPreferencesStore>,
    translate_all_enabled: bool,
}

impl InChatMenuHandler {
    pub fn new(group_prefs: Arc<GroupPreferencesStore>, translate_all_enabled: bool) -> Self {
        Self {
            group_prefs,
            translate_all_enabled,
        }
    }
}

#[async_trait]
impl CommandHandler for InChatMenuHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        is_exact_command(&message.text, "!in-chat")
    }

    fn label(&self) -> &'static str {
        "in_chat_menu"
    }

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        let language = menu_language_for_message(message, &self.group_prefs);
        Ok(in_chat_menu(language, self.translate_all_enabled).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn msg(text: &str) -> BotMessage {
        BotMessage {
            source: "+1".into(),
            source_number: None,
            source_name: None,
            text: text.into(),
            timestamp: 0,
            message_timestamp: 0,
            is_group: true,
            group_id: Some("g-internal".into()),
            group_name: None,
            receiving_account: "+15550001111".into(),
            attachments: vec![],
            quote: None,
        }
    }

    #[test]
    fn menus_match_exact_only() {
        let store = GroupPreferencesStore::new_in_memory(0);
        let t = TranslationMenuHandler::new(store.clone());
        assert!(t.matches(&msg("!translation")));
        assert!(!t.matches(&msg("!translation-on es en")));

        let i = InChatMenuHandler::new(store.clone(), true);
        assert!(i.matches(&msg("!in-chat")));

        let s = TranscriptionPairingHandler::new(
            store.clone(),
            Arc::new(SignalClient::new("http://127.0.0.1:9").unwrap()),
            None,
        );
        assert!(s.matches(&msg("!transcription")));

        let m = TranscriptionMenuHandler::new(store);
        assert!(m.matches(&msg("!transcription")));
    }

    #[tokio::test]
    async fn pairing_without_peer_reports_unavailable() {
        let signal_mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/groups/%2B15550001111"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
                "name": "Main",
                "id": "group.send=",
                "internal_id": "g-internal",
                "members": ["+15550001111"]
            }])))
            .mount(&signal_mock)
            .await;
        Mock::given(method("POST"))
            .and(path("/v2/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount(&signal_mock)
            .await;

        let store = GroupPreferencesStore::new_in_memory(0);
        let handler = TranscriptionPairingHandler::new(
            store,
            Arc::new(SignalClient::new(signal_mock.uri()).unwrap()),
            None,
        );
        let out = handler.execute(&msg("!transcription")).await.unwrap();
        assert!(out.is_empty());
    }

    #[tokio::test]
    async fn pairing_invites_missing_peer() {
        let signal_mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/groups/%2B15550001111"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
                "name": "Main",
                "id": "group.send=",
                "internal_id": "g-internal",
                "members": ["+15550001111"],
                "pending_invites": [],
                "pending_requests": [],
                "admins": ["+15550001111"]
            }])))
            .mount(&signal_mock)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/groups/%2B15550001111/group.send%3D/members"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&signal_mock)
            .await;
        Mock::given(method("POST"))
            .and(path("/v2/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount(&signal_mock)
            .await;

        let store = GroupPreferencesStore::new_in_memory(0);
        let handler = TranscriptionPairingHandler::new(
            store,
            Arc::new(SignalClient::new(signal_mock.uri()).unwrap()),
            Some("+15550009999".into()),
        );
        let out = handler.execute(&msg("!transcription")).await.unwrap();
        assert!(out.is_empty());
    }

    #[tokio::test]
    async fn pairing_silent_when_peer_already_member() {
        let signal_mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/groups/%2B15550001111"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
                "name": "Main",
                "id": "group.send=",
                "internal_id": "g-internal",
                "members": ["+15550001111", "+15550009999"],
                "pending_invites": [],
                "pending_requests": [],
                "admins": ["+15550001111"]
            }])))
            .mount(&signal_mock)
            .await;

        let store = GroupPreferencesStore::new_in_memory(0);
        let handler = TranscriptionPairingHandler::new(
            store,
            Arc::new(SignalClient::new(signal_mock.uri()).unwrap()),
            Some("+15550009999".into()),
        );
        let out = handler.execute(&msg("!transcription")).await.unwrap();
        assert!(out.is_empty());
    }

    #[tokio::test]
    async fn transcription_menu_returns_voice_help() {
        let store = GroupPreferencesStore::new_in_memory(0);
        let handler = TranscriptionMenuHandler::new(store);
        let out = handler.execute(&msg("!transcription")).await.unwrap();
        assert!(out.contains("!transcribe"));
        assert!(out.to_lowercase().contains("voice"));
    }
}
