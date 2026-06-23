//! Implicit voice note handler — transcribe via Whisper and quote-reply.

use crate::commands::CommandHandler;
use crate::error::AppResult;
use async_trait::async_trait;
use signal_client::{Attachment, BotMessage, SignalClient};
use std::sync::Arc;
use tracing::{info, instrument, warn};
use whisper_client::{WhisperClient, WhisperError};

const PROGRESS_MSG: &str = "🎤 Transcribing...";
#[cfg_attr(not(test), allow(dead_code))]
const DEFAULT_REPLY_PREFIX: &str = "📝 Transcript:";

pub struct VoiceHandler {
    whisper: Arc<WhisperClient>,
    signal: Arc<SignalClient>,
    reply_prefix: String,
    max_attachment_bytes: usize,
}

impl VoiceHandler {
    pub fn new(
        whisper: Arc<WhisperClient>,
        signal: Arc<SignalClient>,
        reply_prefix: impl Into<String>,
        max_attachment_bytes: usize,
    ) -> Self {
        Self {
            whisper,
            signal,
            reply_prefix: reply_prefix.into(),
            max_attachment_bytes,
        }
    }

    fn format_transcript(text: &str, prefix: &str) -> String {
        format!("{prefix}\n{text}")
    }

    fn attachment_filename(audio: &Attachment) -> String {
        if let Some(name) = &audio.filename {
            if !name.is_empty() {
                return name.clone();
            }
        }
        if audio.content_type.contains("aac") || audio.content_type.contains("mp4") {
            "voice.m4a".into()
        } else if audio.content_type.contains("ogg") {
            "voice.ogg".into()
        } else {
            "voice.bin".into()
        }
    }

    fn user_message_for_whisper_error(err: &WhisperError) -> &'static str {
        match err {
            WhisperError::EmptyTranscription => {
                "Could not transcribe voice note (no speech detected). Try a clearer recording."
            }
            WhisperError::Http(_) | WhisperError::Api(_) => {
                "Could not transcribe voice note. Try again later."
            }
        }
    }
}

#[async_trait]
impl CommandHandler for VoiceHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        message.is_voice_note()
    }

    fn reply_with_quote(&self) -> bool {
        true
    }

    #[instrument(skip(self, message), fields(source = %message.source, is_group = message.is_group))]
    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        let audio = match message.primary_audio_attachment() {
            Some(a) => a,
            None => return Ok("Could not read voice attachment.".into()),
        };

        if let Some(expected) = audio.size {
            if expected > self.max_attachment_bytes as i64 {
                warn!(
                    expected_bytes = expected,
                    max = self.max_attachment_bytes,
                    "Voice attachment exceeds size limit"
                );
                return Ok(
                    "Voice note too long (max 5 min). Send a shorter clip.".into(),
                );
            }
        }

        if let Err(e) = self
            .signal
            .reply_quoted(message, PROGRESS_MSG, Some("[voice note]"))
            .await
        {
            warn!("Failed to send transcribing progress message: {}", e);
        }

        let bytes = match self.signal.download_attachment(&audio.id).await {
            Ok(bytes) => bytes,
            Err(e) => {
                warn!("Failed to download voice attachment {}: {}", audio.id, e);
                return Ok("Could not download voice note. Try again later.".into());
            }
        };

        if bytes.len() > self.max_attachment_bytes {
            warn!(
                bytes = bytes.len(),
                max = self.max_attachment_bytes,
                "Downloaded voice attachment exceeds size limit"
            );
            return Ok("Voice note too long (max 5 min). Send a shorter clip.".into());
        }

        let filename = Self::attachment_filename(audio);
        let result = self
            .whisper
            .transcribe(&bytes, &filename, &audio.content_type)
            .await;

        match result {
            Ok(transcription) => {
                info!(
                    source = %message.source,
                    chars = transcription.text.len(),
                    "Voice note transcribed"
                );
                Ok(Self::format_transcript(
                    transcription.trimmed_text(),
                    &self.reply_prefix,
                ))
            }
            Err(e) => {
                warn!("Whisper transcription failed: {}", e);
                Ok(Self::user_message_for_whisper_error(&e).into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_transcript_includes_prefix() {
        let out = VoiceHandler::format_transcript("Hola mundo", DEFAULT_REPLY_PREFIX);
        assert_eq!(out, "📝 Transcript:\nHola mundo");
    }

    #[test]
    fn attachment_filename_from_mime() {
        let audio = Attachment {
            content_type: "audio/aac".into(),
            filename: None,
            id: "x".into(),
            size: None,
            upload_timestamp: None,
        };
        assert_eq!(VoiceHandler::attachment_filename(&audio), "voice.m4a");
    }
}
