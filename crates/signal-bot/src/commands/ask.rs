//! `!ask` — explicit opt-in for NEAR AI chat (especially in groups).

use crate::commands::chat::ChatHandler;
use crate::commands::CommandHandler;
use crate::error::AppResult;
use async_trait::async_trait;
use signal_client::BotMessage;

const USAGE: &str = "Usage: !ask <your question>";

pub struct AskHandler {
    chat: ChatHandler,
}

impl AskHandler {
    pub fn new(chat: ChatHandler) -> Self {
        Self { chat }
    }

    fn parse_question(text: &str) -> Option<&str> {
        let trimmed = text.trim();
        if trimmed == "!ask" {
            return None;
        }
        trimmed
            .strip_prefix("!ask")
            .map(|rest| rest.trim())
            .filter(|q| !q.is_empty())
    }
}

#[async_trait]
impl CommandHandler for AskHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        let trimmed = message.text.trim();
        trimmed == "!ask" || trimmed.starts_with("!ask ")
    }

    fn label(&self) -> &'static str {
        "ask"
    }

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        let question = match Self::parse_question(&message.text) {
            Some(q) => q,
            None => return Ok(USAGE.into()),
        };
        self.chat.handle_chat(message, question).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use conversation_store::ConversationStore;
    use near_ai_client::NearAiClient;
    use signal_client::SignalClient;
    use std::sync::Arc;
    use std::time::Duration;
    use tools::ToolRegistry;

    fn handler() -> AskHandler {
        AskHandler::new(ChatHandler::new(
            Arc::new(
                NearAiClient::new("key", "http://localhost", "model", Duration::from_secs(30))
                    .unwrap(),
            ),
            Arc::new(ConversationStore::new(50, Duration::from_secs(3600))),
            Arc::new(SignalClient::new("http://localhost").unwrap()),
            Arc::new(ToolRegistry::new()),
            String::new(),
            5,
            None,
            None,
        ))
    }

    fn sample_message(text: &str) -> BotMessage {
        BotMessage {
            source: "+1234567890".into(),
            text: text.into(),
            timestamp: 0,
            message_timestamp: 0,
            is_group: true,
            group_id: Some("group.test".into()),
            receiving_account: "+0987654321".into(),
            attachments: vec![],
            quote: None,
        }
    }

    #[tokio::test]
    async fn ask_matches_with_question() {
        assert!(handler().matches(&sample_message("!ask what is 2+2?")));
    }

    #[tokio::test]
    async fn ask_matches_bare_command() {
        assert!(handler().matches(&sample_message("!ask")));
    }

    #[tokio::test]
    async fn ask_does_not_match_other_commands() {
        assert!(!handler().matches(&sample_message("!asksomething")));
    }

    #[test]
    fn parse_question_extracts_text() {
        assert_eq!(
            AskHandler::parse_question("!ask hello world"),
            Some("hello world")
        );
    }

    #[test]
    fn parse_question_rejects_bare_command() {
        assert_eq!(AskHandler::parse_question("!ask"), None);
    }
}
