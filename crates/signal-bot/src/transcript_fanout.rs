//! In-process fan-out of transcripts into in-chat + Language Threads.

use crate::commands::{TranslateAllHandler, TranslateMeHandler};
use async_trait::async_trait;
use signal_bot_transcription::TranscriptFanout;
use signal_client::BotMessage;

pub struct SuiteTranscriptFanout {
    pub translate_all: Option<TranslateAllHandler>,
    pub translate_me: TranslateMeHandler,
}

#[async_trait]
impl TranscriptFanout for SuiteTranscriptFanout {
    async fn fan_out_transcript(&self, original: &BotMessage, spoken_text: &str) {
        if let Some(in_chat) = &self.translate_all {
            in_chat.fan_out_transcript(original, spoken_text).await;
        }
        self.translate_me
            .fan_out_transcript(original, spoken_text)
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bot_identity::BotIdentity;
    use crate::commands::translate_lang::resolve_language;
    use crate::group_preferences_store::{GroupPreferencesStore, GroupTranslateMode};
    use near_ai_client::NearAiClient;
    use signal_client::SignalClient;
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn speaker_msg() -> BotMessage {
        BotMessage {
            source: "+alice".into(),
            source_number: Some("+alice".into()),
            source_name: Some("Alice".into()),
            text: String::new(),
            timestamp: 1,
            message_timestamp: 1,
            is_group: true,
            group_id: Some("group.main".into()),
            group_name: None,
            receiving_account: "+15550001111".into(),
            attachments: vec![],
            quote: None,
        }
    }

    async fn send_recipients(signal: &MockServer) -> Vec<String> {
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

    async fn send_messages(signal: &MockServer) -> Vec<String> {
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
    async fn suite_fans_out_to_in_chat_and_language_threads_with_speaker_source() {
        use serde_json::json;

        let signal = MockServer::start().await;
        let near = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&signal)
            .await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "1",
                "choices": [{"index": 0, "message": {"role": "assistant", "content": "Hello"}, "finish_reason": "stop"}],
                "created": 1,
                "model": "m",
                "object": "chat.completion"
            })))
            .mount(&near)
            .await;

        let mode = GroupTranslateMode::new(
            resolve_language("es").unwrap(),
            resolve_language("en").unwrap(),
        );
        let in_chat_store = GroupPreferencesStore::new_in_memory(30);
        in_chat_store.set_member_translate("group.main", "+alice", mode);

        let threads_store = GroupPreferencesStore::new_in_memory(0);
        threads_store.set_sidecar("group.main", "es", "group.es".into(), "es-internal".into());

        let near_ai = Arc::new(
            NearAiClient::new("key", near.uri(), "m", std::time::Duration::from_secs(5)).unwrap(),
        );
        let signal_client = Arc::new(SignalClient::new(signal.uri()).unwrap());
        let suite = SuiteTranscriptFanout {
            translate_all: Some(TranslateAllHandler::new(
                in_chat_store,
                near_ai.clone(),
                signal_client.clone(),
            )),
            translate_me: TranslateMeHandler::new(
                threads_store,
                near_ai,
                signal_client,
                BotIdentity::new(),
            ),
        };

        suite
            .fan_out_transcript(&speaker_msg(), "Hola amigos")
            .await;

        let near_hits = near
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .filter(|r| r.url.path() == "/chat/completions")
            .count();
        assert!(near_hits >= 1, "in-chat should call NEAR to translate");

        let recipients = send_recipients(&signal).await;
        assert!(
            recipients.contains(&"group.main".to_string()),
            "in-chat quote-reply should send to the main group: {recipients:?}"
        );
        assert!(
            recipients.contains(&"group.es".to_string()),
            "Language Threads should relay to the sidecar: {recipients:?}"
        );

        let messages = send_messages(&signal).await;
        assert!(
            messages.iter().any(|m| m.starts_with("Alice:\n")),
            "sidecar attribution should use the original speaker: {messages:?}"
        );
    }
}
