//! OpenAI-compatible audio transcription client (NEAR AI Whisper Large V3).

use crate::error::WhisperError;
use crate::types::{
    whisper_language_to_iso, HealthResponse, InferenceResponse, TranscriptionResult,
};
use reqwest::Client;
use std::time::Duration;
use tracing::{debug, instrument, warn};

/// Client for `POST /audio/transcriptions` (OpenAI-compatible, e.g. NEAR AI).
#[derive(Clone)]
pub struct WhisperClient {
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl WhisperClient {
    /// Create a client. `base_url` is the API root including `/v1` when required.
    pub fn new(
        base_url: impl Into<String>,
        timeout: Duration,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Result<Self, WhisperError> {
        let client = Client::builder().timeout(timeout).build()?;

        Ok(Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key: api_key.into(),
            model: model.into(),
        })
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    /// Check if the STT API is reachable (`GET /models`).
    pub async fn health_check(&self) -> bool {
        self.client
            .get(format!("{}/models", self.base_url))
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Fetch a coarse health status from `GET /models`.
    #[instrument(skip(self))]
    pub async fn health(&self) -> Result<HealthResponse, WhisperError> {
        let response = self
            .client
            .get(format!("{}/models", self.base_url))
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if !response.status().is_success() {
            let msg = response.text().await.unwrap_or_default();
            return Err(WhisperError::Api(msg));
        }

        Ok(HealthResponse {
            status: "ok".into(),
        })
    }

    /// Transcribe audio bytes via `POST /audio/transcriptions`.
    #[instrument(skip(self, audio))]
    pub async fn transcribe(
        &self,
        audio: &[u8],
        filename: &str,
        content_type: &str,
    ) -> Result<TranscriptionResult, WhisperError> {
        self.upload(audio, filename, content_type, false).await
    }

    /// Translate speech to English via `POST /audio/translations`.
    #[instrument(skip(self, audio))]
    pub async fn translate_to_english(
        &self,
        audio: &[u8],
        filename: &str,
        content_type: &str,
    ) -> Result<TranscriptionResult, WhisperError> {
        self.upload(audio, filename, content_type, true).await
    }

    async fn upload(
        &self,
        audio: &[u8],
        filename: &str,
        content_type: &str,
        translate: bool,
    ) -> Result<TranscriptionResult, WhisperError> {
        let mut part =
            reqwest::multipart::Part::bytes(audio.to_vec()).file_name(filename.to_string());
        if !content_type.is_empty() {
            part = part.mime_str(content_type).map_err(WhisperError::Http)?;
        }

        let form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("model", self.model.clone())
            .text("response_format", "json");

        let path = if translate {
            "audio/translations"
        } else {
            "audio/transcriptions"
        };

        let response = self
            .client
            .post(format!("{}/{path}", self.base_url))
            .header("Authorization", self.auth_header())
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            let msg = response.text().await.unwrap_or_default();
            warn!("Whisper {path} failed: {msg}");
            return Err(WhisperError::Api(msg));
        }

        let body: InferenceResponse = response.json().await?;
        let text = body.text.trim().to_string();
        if text.is_empty() {
            return Err(WhisperError::EmptyTranscription);
        }

        let language = body
            .language
            .as_deref()
            .or(body.detected_language.as_deref())
            .and_then(whisper_language_to_iso)
            .map(str::to_string);

        debug!(
            "Whisper {} complete ({} chars, lang={:?})",
            if translate { "translate" } else { "transcribe" },
            text.len(),
            language
        );

        Ok(TranscriptionResult { text, language })
    }
}
