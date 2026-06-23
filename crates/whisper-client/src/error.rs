//! Whisper client errors.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum WhisperError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API error: {0}")]
    Api(String),

    #[error("Empty transcription")]
    EmptyTranscription,
}
