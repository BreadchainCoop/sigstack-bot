//! Product menus: `!translation-threads`, `!translation-in-chat`, `!transcription`, redirects.

use crate::commands::menu_locale::{
    help_in_chat_guide, help_menu, help_threads_guide, help_transcription_guide, is_exact_command,
    is_translation_in_chat_menu_command, is_translation_threads_menu_command,
    transcription_group_only, transcription_invited, transcription_unavailable,
    translation_in_chat_menu, translation_split_redirect, translation_threads_menu,
};
use crate::commands::CommandHandler;
use crate::config::BotRole;
use crate::error::AppResult;
use async_trait::async_trait;
use signal_client::{BotMessage, SignalClient};
use std::sync::Arc;
use tracing::warn;

/// Legacy `!translation` → points at the two product menus.
pub struct TranslationMenuHandler;

impl TranslationMenuHandler {
    pub fn new(_translate_all_enabled: bool) -> Self {
        Self
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

    async fn execute(&self, _message: &BotMessage) -> AppResult<String> {
        Ok(translation_split_redirect().into())
    }
}

pub struct TranslationThreadsMenuHandler;

impl TranslationThreadsMenuHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TranslationThreadsMenuHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandHandler for TranslationThreadsMenuHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        is_translation_threads_menu_command(&message.text)
    }

    fn label(&self) -> &'static str {
        "translation_threads_menu"
    }

    async fn execute(&self, _message: &BotMessage) -> AppResult<String> {
        Ok(translation_threads_menu().into())
    }
}

pub struct TranslationInChatMenuHandler {
    translate_all_enabled: bool,
}

impl TranslationInChatMenuHandler {
    pub fn new(translate_all_enabled: bool) -> Self {
        Self {
            translate_all_enabled,
        }
    }
}

#[async_trait]
impl CommandHandler for TranslationInChatMenuHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        is_translation_in_chat_menu_command(&message.text)
    }

    fn label(&self) -> &'static str {
        "translation_in_chat_menu"
    }

    async fn execute(&self, _message: &BotMessage) -> AppResult<String> {
        Ok(translation_in_chat_menu(self.translate_all_enabled).into())
    }
}

/// Translation role: invite the transcription peer, or stay silent when already paired.
pub struct TranscriptionPairingHandler {
    signal: Arc<SignalClient>,
    peer_phone: Option<String>,
}

impl TranscriptionPairingHandler {
    pub fn new(signal: Arc<SignalClient>, peer_phone: Option<String>) -> Self {
        Self {
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
        if !message.is_group {
            self.send(message, transcription_group_only()).await?;
            return Ok(String::new());
        }

        let Some(peer) = self.peer_phone.as_deref() else {
            self.send(message, transcription_unavailable()).await?;
            return Ok(String::new());
        };

        let Some(group_id) = message.group_id.as_deref() else {
            self.send(message, transcription_unavailable()).await?;
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
            self.send(message, transcription_unavailable()).await?;
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
                self.send(message, transcription_invited()).await?;
            }
            Err(e) => {
                warn!(error = %e, peer, "Failed to invite transcription bot");
                let body = format!(
                    "Could not add the transcription bot ({peer}): {e}\n\n\
                     This bot must be a group admin to invite members. \
                     Or set SIGNAL__PEER_PHONE and try again.\n\n\
                     !help\n  Main menu"
                );
                self.send(message, &body).await?;
            }
        }

        Ok(String::new())
    }
}

/// Transcription role: product menu for `!transcription`.
pub struct TranscriptionMenuHandler;

impl TranscriptionMenuHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TranscriptionMenuHandler {
    fn default() -> Self {
        Self::new()
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

    async fn execute(&self, _message: &BotMessage) -> AppResult<String> {
        Ok(help_menu(BotRole::Transcription).into())
    }
}

pub struct InChatMenuHandler {
    translate_all_enabled: bool,
}

impl InChatMenuHandler {
    pub fn new(translate_all_enabled: bool) -> Self {
        Self {
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

    async fn execute(&self, _message: &BotMessage) -> AppResult<String> {
        Ok(translation_in_chat_menu(self.translate_all_enabled).into())
    }
}

/// Feature guide: how Language Threads works.
pub struct HelpThreadsHandler;

impl HelpThreadsHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HelpThreadsHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandHandler for HelpThreadsHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        is_exact_command(&message.text, "!help-threads")
    }

    fn label(&self) -> &'static str {
        "help_threads"
    }

    async fn execute(&self, _message: &BotMessage) -> AppResult<String> {
        Ok(help_threads_guide().into())
    }
}

/// Feature guide: how in-chat translation works.
pub struct HelpInChatHandler;

impl HelpInChatHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HelpInChatHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandHandler for HelpInChatHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        is_exact_command(&message.text, "!help-in-chat")
    }

    fn label(&self) -> &'static str {
        "help_in_chat"
    }

    async fn execute(&self, _message: &BotMessage) -> AppResult<String> {
        Ok(help_in_chat_guide().into())
    }
}

/// Feature guide: how voice transcription works.
pub struct HelpTranscriptionHandler;

impl HelpTranscriptionHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HelpTranscriptionHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandHandler for HelpTranscriptionHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        is_exact_command(&message.text, "!help-transcription")
    }

    fn label(&self) -> &'static str {
        "help_transcription"
    }

    async fn execute(&self, _message: &BotMessage) -> AppResult<String> {
        Ok(help_transcription_guide().into())
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
        let t = TranslationMenuHandler::new(true);
        assert!(t.matches(&msg("!translation")));
        assert!(!t.matches(&msg("!translation-on es en")));

        let threads = TranslationThreadsMenuHandler::new();
        assert!(threads.matches(&msg("!translation-threads")));
        assert!(threads.matches(&msg("!translate-threads")));
        assert!(threads.matches(&msg("!translate-thread")));
        assert!(threads.matches(&msg("!translation-thread")));
        assert!(!threads.matches(&msg("!translation-on es en")));

        let in_chat_prod = TranslationInChatMenuHandler::new(true);
        assert!(in_chat_prod.matches(&msg("!translation-in-chat")));
        assert!(in_chat_prod.matches(&msg("!translate-in-chat")));
        assert!(!in_chat_prod.matches(&msg("!translation-on es en")));

        let i = InChatMenuHandler::new(true);
        assert!(i.matches(&msg("!in-chat")));

        let ht = HelpThreadsHandler::new();
        assert!(ht.matches(&msg("!help-threads")));
        assert!(!ht.matches(&msg("!help")));

        let hi = HelpInChatHandler::new();
        assert!(hi.matches(&msg("!help-in-chat")));
        assert!(!hi.matches(&msg("!help")));

        let htr = HelpTranscriptionHandler::new();
        assert!(htr.matches(&msg("!help-transcription")));
        assert!(!htr.matches(&msg("!help")));
        assert!(!htr.matches(&msg("!transcription")));

        let s = TranscriptionPairingHandler::new(
            Arc::new(SignalClient::new("http://127.0.0.1:9").unwrap()),
            None,
        );
        assert!(s.matches(&msg("!transcription")));

        let m = TranscriptionMenuHandler::new();
        assert!(m.matches(&msg("!transcription")));
    }

    #[tokio::test]
    async fn in_chat_typo_aliases_match_canonical_menu() {
        let canonical = TranslationInChatMenuHandler::new(true);
        let expected = canonical
            .execute(&msg("!translation-in-chat"))
            .await
            .unwrap();
        for typo in ["!translate-in-chat"] {
            let got = canonical.execute(&msg(typo)).await.unwrap();
            assert_eq!(got, expected);
        }
    }

    #[tokio::test]
    async fn threads_typo_aliases_match_canonical_menu() {
        let handler = TranslationThreadsMenuHandler::new();
        let expected = handler.execute(&msg("!translation-threads")).await.unwrap();
        for typo in [
            "!translate-threads",
            "!translate-thread",
            "!translation-thread",
        ] {
            let got = handler.execute(&msg(typo)).await.unwrap();
            assert_eq!(got, expected);
        }
    }

    #[tokio::test]
    async fn in_chat_alias_matches_in_chat_menu() {
        let product = TranslationInChatMenuHandler::new(true);
        let alias = InChatMenuHandler::new(true);
        let via_product = product.execute(&msg("!translation-in-chat")).await.unwrap();
        let via_alias = alias.execute(&msg("!in-chat")).await.unwrap();
        assert_eq!(via_product, via_alias);
        assert!(via_product.contains("!translate-all-on"));
        assert!(via_product.contains("!translate-me-on"));
        assert!(!via_product.contains("!translate-me-thread"));
    }

    #[tokio::test]
    async fn translation_redirect_names_both_menus() {
        let translation = TranslationMenuHandler::new(true);
        let out = translation.execute(&msg("!translation")).await.unwrap();
        assert!(out.contains("!translation-threads"));
        assert!(out.contains("!translation-in-chat"));
    }

    #[tokio::test]
    async fn feature_guides_return_use_case_copy() {
        let threads = HelpThreadsHandler::new()
            .execute(&msg("!help-threads"))
            .await
            .unwrap();
        assert!(threads.contains("sidecar"));
        assert!(threads.contains("!translate-me-thread"));

        let in_chat = HelpInChatHandler::new()
            .execute(&msg("!help-in-chat"))
            .await
            .unwrap();
        assert!(in_chat.contains("quote"));
        assert!(in_chat.contains("!translate-all-on"));

        let transcription = HelpTranscriptionHandler::new()
            .execute(&msg("!help-transcription"))
            .await
            .unwrap();
        assert!(transcription.contains("Whisper"));
        assert!(transcription.contains("!transcribe"));
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

        let handler = TranscriptionPairingHandler::new(
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

        let handler = TranscriptionPairingHandler::new(
            Arc::new(SignalClient::new(signal_mock.uri()).unwrap()),
            Some("+15550009999".into()),
        );
        let out = handler.execute(&msg("!transcription")).await.unwrap();
        assert!(out.is_empty());
    }
}
