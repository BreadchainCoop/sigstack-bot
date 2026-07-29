//! `!rename <name>` — rename a Language Thread sidecar (members only via presence).

use crate::commands::CommandHandler;
use crate::error::AppResult;
use crate::group_preferences_store::GroupPreferencesStore;
use async_trait::async_trait;
use signal_client::{BotMessage, SignalClient};
use std::sync::Arc;
use tracing::warn;

const MAX_NAME_LEN: usize = 100;
const NOT_THREAD_MSG: &str = "!rename is only available in a Language Thread.";
const USAGE_MSG: &str = "Usage: !rename <name>";
const EMPTY_MSG: &str = "Group name cannot be empty.";
const TOO_LONG_MSG: &str = "Group name is too long (max 100 characters).";

pub struct RenameHandler {
    store: Arc<GroupPreferencesStore>,
    signal: Arc<SignalClient>,
}

impl RenameHandler {
    pub fn new(store: Arc<GroupPreferencesStore>, signal: Arc<SignalClient>) -> Self {
        Self { store, signal }
    }

    fn parse_name(text: &str) -> Option<&str> {
        let t = text.trim();
        let rest = t.strip_prefix("!rename")?;
        if !rest.is_empty() && !rest.starts_with(' ') && !rest.starts_with('\t') {
            return None;
        }
        Some(rest.trim())
    }
}

#[async_trait]
impl CommandHandler for RenameHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        let t = message.text.trim();
        t == "!rename" || t.starts_with("!rename ")
    }

    fn label(&self) -> &'static str {
        "rename"
    }

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        let Some(name_arg) = Self::parse_name(&message.text) else {
            return Ok(USAGE_MSG.into());
        };
        if name_arg.is_empty() {
            return Ok(format!("{EMPTY_MSG}\n{USAGE_MSG}"));
        }
        if name_arg.chars().count() > MAX_NAME_LEN {
            return Ok(TOO_LONG_MSG.into());
        }

        let Some(group_id) = message.group_id.as_deref() else {
            return Ok(NOT_THREAD_MSG.into());
        };

        let Some((main_id, lang)) = self.store.lookup_sidecar(group_id) else {
            return Ok(NOT_THREAD_MSG.into());
        };

        let Some(send_id) = self
            .store
            .get_bridge(&main_id)
            .and_then(|b| b.sidecar_send_id(&lang).map(str::to_string))
        else {
            return Ok(NOT_THREAD_MSG.into());
        };

        let bot = message.receiving_account.as_str();
        match self.signal.update_group(bot, &send_id, name_arg).await {
            Ok(()) => Ok(format!("Renamed this Language Thread to \"{name_arg}\".")),
            Err(e) => {
                warn!(error = %e, send_id, "Failed to rename Language Thread");
                Ok(format!(
                    "Could not rename this group: {e}. Try again shortly."
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn group_msg(text: &str, group_id: &str) -> BotMessage {
        BotMessage {
            source: "+15550002222".into(),
            source_number: Some("+15550002222".into()),
            source_name: Some("Maria".into()),
            text: text.into(),
            timestamp: 1,
            message_timestamp: 1,
            is_group: true,
            group_id: Some(group_id.into()),
            group_name: None,
            receiving_account: "+15550001111".into(),
            attachments: vec![],
            quote: None,
        }
    }

    #[test]
    fn matches_rename_commands() {
        let store = GroupPreferencesStore::new_in_memory(0);
        let signal = Arc::new(SignalClient::new("http://127.0.0.1:9").unwrap());
        let handler = RenameHandler::new(store, signal);
        assert!(handler.matches(&group_msg("!rename Foo", "g")));
        assert!(handler.matches(&group_msg("!rename", "g")));
        assert!(!handler.matches(&group_msg("!renamex", "g")));
        assert!(!handler.matches(&group_msg("!help", "g")));
    }

    #[tokio::test]
    async fn rejects_outside_sidecar() {
        let store = GroupPreferencesStore::new_in_memory(0);
        let signal = Arc::new(SignalClient::new("http://127.0.0.1:9").unwrap());
        let handler = RenameHandler::new(store, signal);
        let out = handler
            .execute(&group_msg("!rename New Name", "unknown"))
            .await
            .unwrap();
        assert!(out.contains("only available"));
    }

    #[tokio::test]
    async fn renames_known_sidecar() {
        let mock = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/v1/groups/%2B15550001111/group.it%3D%3D"))
            .and(body_json(json!({ "name": "Il nostro filo" })))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&mock)
            .await;

        let store = GroupPreferencesStore::new_in_memory(0);
        store.set_sidecar("main-1", "it", "group.it==".into(), "it-internal".into());
        let handler = RenameHandler::new(store, Arc::new(SignalClient::new(mock.uri()).unwrap()));
        let out = handler
            .execute(&group_msg("!rename Il nostro filo", "it-internal"))
            .await
            .unwrap();
        assert!(out.contains("Il nostro filo"));
    }

    #[tokio::test]
    async fn rejects_empty_and_overlong() {
        let store = GroupPreferencesStore::new_in_memory(0);
        store.set_sidecar("main-1", "it", "group.it==".into(), "it-internal".into());
        let signal = Arc::new(SignalClient::new("http://127.0.0.1:9").unwrap());
        let handler = RenameHandler::new(store, signal);

        let empty = handler
            .execute(&group_msg("!rename   ", "it-internal"))
            .await
            .unwrap();
        assert!(empty.contains("cannot be empty"));

        let long = format!("!rename {}", "x".repeat(101));
        let too_long = handler
            .execute(&group_msg(&long, "it-internal"))
            .await
            .unwrap();
        assert!(too_long.contains("too long"));
    }
}
