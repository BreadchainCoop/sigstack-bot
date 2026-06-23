//! Whisper API types.

use serde::Deserialize;

/// Response from `GET /health`.
#[derive(Debug, Clone, Deserialize)]
pub struct HealthResponse {
    pub status: String,
}

/// JSON response from `POST /inference` with `response_format=json`.
#[derive(Debug, Clone, Deserialize)]
pub struct InferenceResponse {
    pub text: String,
}

/// Parsed transcription result.
#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    pub text: String,
}

impl TranscriptionResult {
    pub fn trimmed_text(&self) -> &str {
        self.text.trim()
    }
}
