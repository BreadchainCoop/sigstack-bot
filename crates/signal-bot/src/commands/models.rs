//! Models command - lists available AI models.

use crate::commands::CommandHandler;
use crate::error::AppResult;
use async_trait::async_trait;
use near_ai_client::NearAiClient;
use signal_client::BotMessage;
use std::sync::Arc;
use tracing::error;

pub struct ModelsHandler {
    near_ai: Arc<NearAiClient>,
}

impl ModelsHandler {
    pub fn new(near_ai: Arc<NearAiClient>) -> Self {
        Self { near_ai }
    }
}

#[async_trait]
impl CommandHandler for ModelsHandler {
    fn trigger(&self) -> Option<&str> {
        Some("!models")
    }

    async fn execute(&self, _message: &BotMessage) -> AppResult<String> {
        match self.near_ai.list_models().await {
            Ok(models) => {
                let model_list: String = models
                    .iter()
                    .take(10)
                    .map(|m| format!("- {}", m.id))
                    .collect::<Vec<_>>()
                    .join("\n");

                Ok(format!(
                    "**Available Models:**\n{}\n\n_Current: {}_",
                    model_list,
                    self.near_ai.model()
                ))
            }
            Err(e) => {
                error!("Failed to list models: {}", e);
                Ok("Could not fetch model list.".into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn dm() -> BotMessage {
        BotMessage {
            source: "+1".into(),
            source_number: None,
            source_name: None,
            text: "!models".into(),
            timestamp: 0,
            message_timestamp: 0,
            is_group: false,
            group_id: None,
            group_name: None,
            receiving_account: "+2".into(),
            attachments: vec![],
            quote: None,
        }
    }

    #[tokio::test]
    async fn models_lists_known_near_ai_models() {
        let near = Arc::new(
            NearAiClient::new(
                "key",
                "http://127.0.0.1:9",
                "test-model",
                Duration::from_secs(2),
            )
            .unwrap(),
        );
        let handler = ModelsHandler::new(near);
        assert!(handler.matches(&dm()));
        let out = handler.execute(&dm()).await.unwrap();
        assert!(out.contains("**Available Models:**"));
        assert!(out.contains("deepseek-ai/DeepSeek-V3.1"));
        assert!(out.contains("_Current: test-model_"));
    }
}
