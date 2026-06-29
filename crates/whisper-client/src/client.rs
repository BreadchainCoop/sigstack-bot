//! whisper.cpp HTTP server client.

use crate::error::WhisperError;
use crate::types::{whisper_language_to_iso, HealthResponse, InferenceResponse, TranscriptionResult};
use reqwest::Client;
use std::time::Duration;
use tracing::{debug, instrument, warn};

/// Client for whisper.cpp `whisper-server` (`/health`, `/inference`).
#[derive(Clone)]
pub struct WhisperClient {
    client: Client,
    base_url: String,
}

impl WhisperClient {
    /// Create a new Whisper client.
    pub fn new(base_url: impl Into<String>, timeout: Duration) -> Result<Self, WhisperError> {
        let client = Client::builder().timeout(timeout).build()?;

        Ok(Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
        })
    }

    /// Check if the Whisper API is healthy.
    pub async fn health_check(&self) -> bool {
        self.client
            .get(format!("{}/health", self.base_url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Fetch health response (includes status string when available).
    #[instrument(skip(self))]
    pub async fn health(&self) -> Result<HealthResponse, WhisperError> {
        let response = self
            .client
            .get(format!("{}/health", self.base_url))
            .send()
            .await?;

        if !response.status().is_success() {
            let msg = response.text().await.unwrap_or_default();
            return Err(WhisperError::Api(msg));
        }

        Ok(response.json().await?)
    }

    /// Transcribe audio bytes via `POST /inference` (multipart upload).
    #[instrument(skip(self, audio))]
    pub async fn transcribe(
        &self,
        audio: &[u8],
        filename: &str,
        content_type: &str,
    ) -> Result<TranscriptionResult, WhisperError> {
        self.inference(audio, filename, content_type, false).await
    }

    /// Translate speech to English via `POST /inference` with translate enabled.
    #[instrument(skip(self, audio))]
    pub async fn translate_to_english(
        &self,
        audio: &[u8],
        filename: &str,
        content_type: &str,
    ) -> Result<TranscriptionResult, WhisperError> {
        self.inference(audio, filename, content_type, true).await
    }

    async fn inference(
        &self,
        audio: &[u8],
        filename: &str,
        content_type: &str,
        translate: bool,
    ) -> Result<TranscriptionResult, WhisperError> {
        let mut part = reqwest::multipart::Part::bytes(audio.to_vec()).file_name(filename.to_string());
        if !content_type.is_empty() {
            part = part.mime_str(content_type).map_err(WhisperError::Http)?;
        }

        let mut form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("response_format", "verbose_json")
            .text("language", "auto");

        if translate {
            form = form.text("translate", "true");
        } else {
            form = form.text("translate", "false");
        }

        let response = self
            .client
            .post(format!("{}/inference", self.base_url))
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            let msg = response.text().await.unwrap_or_default();
            warn!("Whisper inference failed: {}", msg);
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
