//! HTTP client for OpenAI-compatible audio transcription (NEAR AI Whisper).

mod client;
mod error;
mod types;

pub use client::WhisperClient;
pub use error::WhisperError;
pub use types::{whisper_language_to_iso, HealthResponse, TranscriptionResult};

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn test_client(server: &MockServer) -> WhisperClient {
        WhisperClient::new(
            server.uri(),
            Duration::from_secs(5),
            "test-key",
            "openai/whisper-large-v3",
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_health_check_success() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": "ok"
            })))
            .mount(&server)
            .await;

        let client = test_client(&server).await;
        assert!(client.health_check().await);
    }

    #[tokio::test]
    async fn test_transcribe_multipart() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/audio/transcriptions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "text": " Hello world\n",
                "language": "english"
            })))
            .mount(&server)
            .await;

        let client = test_client(&server).await;
        let result = client
            .transcribe(b"fake-audio", "note.m4a", "audio/aac")
            .await
            .unwrap();

        assert_eq!(result.trimmed_text(), "Hello world");
        assert_eq!(result.language.as_deref(), Some("en"));

        let received = server.received_requests().await.unwrap();
        let body = String::from_utf8_lossy(&received[0].body);
        assert!(body.contains("filename=\"note.m4a\""));
        assert!(body.contains("openai/whisper-large-v3"));
        assert!(!body.contains("+1555"));
        assert!(!body.contains("group_id"));
        assert!(!body.contains("display_name"));
    }

    #[tokio::test]
    async fn test_transcribe_empty_fails() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/audio/transcriptions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "text": "   \n"
            })))
            .mount(&server)
            .await;

        let client = test_client(&server).await;
        let err = client
            .transcribe(b"fake-audio", "note.m4a", "audio/aac")
            .await
            .unwrap_err();

        assert!(matches!(err, WhisperError::EmptyTranscription));
    }

    #[tokio::test]
    async fn health_endpoint_parses_status() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": "ok"
            })))
            .mount(&server)
            .await;
        let client = test_client(&server).await;
        let health = client.health().await.unwrap();
        assert_eq!(health.status, "ok");
    }

    #[tokio::test]
    async fn health_endpoint_http_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(ResponseTemplate::new(503).set_body_string("down"))
            .mount(&server)
            .await;
        let client = test_client(&server).await;
        assert!(matches!(
            client.health().await.unwrap_err(),
            WhisperError::Api(_)
        ));
    }

    #[tokio::test]
    async fn translate_to_english_uses_translations() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/audio/translations"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "text": " Hello from Spanish\n",
                "language": "spanish"
            })))
            .mount(&server)
            .await;
        let client = test_client(&server).await;
        let result = client
            .translate_to_english(b"audio", "n.m4a", "audio/aac")
            .await
            .unwrap();
        assert_eq!(result.trimmed_text(), "Hello from Spanish");
        assert_eq!(result.language.as_deref(), Some("es"));
    }

    #[test]
    fn whisper_language_to_iso_covers_common_names() {
        assert_eq!(whisper_language_to_iso("English"), Some("en"));
        assert_eq!(whisper_language_to_iso("es"), Some("es"));
        assert_eq!(whisper_language_to_iso("castilian"), Some("es"));
        assert_eq!(whisper_language_to_iso("portuguese"), Some("pt"));
        assert_eq!(whisper_language_to_iso("unknown-lang"), None);
    }
}
