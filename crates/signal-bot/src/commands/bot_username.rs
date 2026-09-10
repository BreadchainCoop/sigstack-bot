//! `!bot-username` — secret ops command: return live Signal username + share token.

use crate::commands::CommandHandler;
use crate::ensure_username::{claim_signal_username, username_link_token, ClaimUsernameError};
use crate::error::AppResult;
use async_trait::async_trait;
use signal_bot_core::is_exact_command_any;
use signal_client::{BotMessage, SignalClient};
use std::sync::Arc;
use tracing::warn;

const BOT_USERNAME_COMMANDS: &[&str] = &["!bot-username"];
const NO_PHONE_MSG: &str = "Could not determine this bot's Signal phone number.";
const EMPTY_NICK_MSG: &str =
    "BOT__SIGNAL_USERNAME is empty; set a nickname (e.g. sigstack) and retry.";
const UNKNOWN_USER_MSG: &str = "Signal returned no username. Try again shortly.";
const NO_TOKEN_MSG: &str =
    "Signal returned no share token. Try again shortly, then set PUBLIC_SIGNAL_USERNAME_TOKEN.";

pub struct BotUsernameHandler {
    signal: Arc<SignalClient>,
    configured_phone: Option<String>,
    nickname: String,
}

impl BotUsernameHandler {
    pub fn new(
        signal: Arc<SignalClient>,
        configured_phone: Option<String>,
        nickname: String,
    ) -> Self {
        Self {
            signal,
            configured_phone,
            nickname,
        }
    }

    fn resolve_phone(&self, message: &BotMessage) -> Option<String> {
        let from_msg = message.receiving_account.trim();
        if !from_msg.is_empty() {
            return Some(from_msg.to_string());
        }
        self.configured_phone
            .as_deref()
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(str::to_string)
    }

    /// Username on line 1; site token on line 2 (never the full signal.me URL).
    fn format_reply(username: &str, token: &str) -> String {
        format!("{username}\n{token}")
    }
}

#[async_trait]
impl CommandHandler for BotUsernameHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        is_exact_command_any(&message.text, BOT_USERNAME_COMMANDS)
    }

    fn label(&self) -> &'static str {
        "bot_username"
    }

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        let Some(phone) = self.resolve_phone(message) else {
            return Ok(NO_PHONE_MSG.into());
        };

        match claim_signal_username(&self.signal, &phone, &self.nickname).await {
            Ok(info) => {
                let Some(username) = info.username.as_deref().filter(|u| !u.is_empty()) else {
                    return Ok(UNKNOWN_USER_MSG.into());
                };
                let Some(token) = info.username_link.as_deref().and_then(username_link_token)
                else {
                    return Ok(format!("{username}\n{NO_TOKEN_MSG}"));
                };
                Ok(Self::format_reply(username, token))
            }
            Err(ClaimUsernameError::EmptyNickname) => Ok(EMPTY_NICK_MSG.into()),
            Err(err) => {
                warn!(error = %err, "Failed to claim Signal username for !bot-username");
                Ok(format!(
                    "Could not fetch Signal username: {err}. Try again shortly."
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::menu_locale::{
        help_menu, thread_help_menu, translation_in_chat_menu, translation_threads_menu,
    };
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn dm(text: &str) -> BotMessage {
        BotMessage {
            source: "+15550002222".into(),
            source_number: Some("+15550002222".into()),
            source_name: Some("Dev".into()),
            text: text.into(),
            timestamp: 1,
            message_timestamp: 1,
            is_group: false,
            group_id: None,
            group_name: None,
            receiving_account: "+15550001111".into(),
            attachments: vec![],
            quote: None,
        }
    }

    fn handler(signal: Arc<SignalClient>, nick: &str) -> BotUsernameHandler {
        BotUsernameHandler::new(signal, Some("+15550001111".into()), nick.into())
    }

    #[test]
    fn matches_exact_bot_username() {
        let signal = Arc::new(SignalClient::new("http://127.0.0.1:9").unwrap());
        let h = handler(signal, "sigstack");
        assert!(h.matches(&dm("!bot-username")));
        assert!(h.matches(&dm("  !bot-username  ")));
        assert!(h.matches(&dm("!BOT_USERNAME")));
        assert!(!h.matches(&dm("!bot-username-extra")));
        assert!(!h.matches(&dm("!bot-username now")));
        assert!(!h.matches(&dm("!help")));
    }

    #[tokio::test]
    async fn returns_username_and_token() {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/accounts/%2B15550001111/username"))
            .and(body_json(serde_json::json!({ "username": "sigstack" })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "username": "sigstack.57",
                "username_link": "https://signal.me/#eu/abc"
            })))
            .expect(1)
            .mount(&mock)
            .await;

        let h = handler(
            Arc::new(SignalClient::new(mock.uri()).unwrap()),
            "sigstack.99",
        );
        let out = h.execute(&dm("!bot-username")).await.unwrap();
        assert_eq!(out, "sigstack.57\nabc");
        assert!(!out.contains("https://signal.me"));
    }

    #[tokio::test]
    async fn returns_friendly_error_on_api_failure() {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/accounts/%2B15550001111/username"))
            .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
            .expect(1)
            .mount(&mock)
            .await;

        let h = handler(Arc::new(SignalClient::new(mock.uri()).unwrap()), "sigstack");
        let out = h.execute(&dm("!bot-username")).await.unwrap();
        assert!(out.contains("Could not fetch Signal username"));
    }

    #[tokio::test]
    async fn empty_nickname_message() {
        let signal = Arc::new(SignalClient::new("http://127.0.0.1:9").unwrap());
        let h = handler(signal, "  ");
        let out = h.execute(&dm("!bot-username")).await.unwrap();
        assert!(out.contains("BOT__SIGNAL_USERNAME"));
    }

    #[test]
    fn menus_omit_bot_username() {
        assert!(!help_menu().contains("!bot-username"));
        assert!(!thread_help_menu().contains("!bot-username"));
        assert!(!translation_threads_menu().contains("!bot-username"));
        assert!(!translation_in_chat_menu(true).contains("!bot-username"));
        assert!(!translation_in_chat_menu(false).contains("!bot-username"));
    }
}
