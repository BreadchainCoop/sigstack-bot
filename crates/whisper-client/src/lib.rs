//! HTTP client for whisper.cpp `whisper-server` sidecar.

mod client;
mod error;
mod types;

pub use client::WhisperClient;
pub use error::WhisperError;
pub use types::{HealthResponse, TranscriptionResult, whisper_language_to_iso};

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn test_client(server: &MockServer) -> WhisperClient {
        WhisperClient::new(server.uri(), Duration::from_secs(5)).unwrap()
    }

    #[tokio::test]
    async fn test_health_check_success() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
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
            .and(path("/inference"))
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
    }

    #[tokio::test]
    async fn test_transcribe_empty_fails() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/inference"))
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
}
