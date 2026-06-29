//! `!transcribe` — quote-reply manual voice transcription via Whisper.

use crate::commands::voice::VoiceHandler;
use crate::commands::CommandHandler;
use crate::error::AppResult;
use async_trait::async_trait;
use signal_client::{Attachment, BotMessage, QuotedMessage, SignalClient};
use std::sync::Arc;
use tracing::{info, instrument, warn};
use whisper_client::{WhisperClient, WhisperError};

pub struct ManualTranscribeHandler {
    whisper: Arc<WhisperClient>,
    signal: Arc<SignalClient>,
    reply_prefix: String,
    max_attachment_bytes: usize,
}

impl ManualTranscribeHandler {
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

    fn quote_author(quote: &QuotedMessage) -> Option<&str> {
        quote.author_number.as_deref()
    }

    fn truncate_snippet(text: &str, max_len: usize) -> String {
        if text.chars().count() <= max_len {
            text.to_string()
        } else {
            let truncated: String = text.chars().take(max_len).collect();
            format!("{truncated}…")
        }
    }

    async fn send_reply(
        &self,
        message: &BotMessage,
        quote: Option<&QuotedMessage>,
        body: &str,
    ) -> AppResult<()> {
        if let Some(quote) = quote {
            let author = Self::quote_author(quote).unwrap_or(message.quote_author());
            let snippet = quote
                .text
                .as_deref()
                .map(|t| Self::truncate_snippet(t, 120))
                .or_else(|| quote.audio_attachment.as_ref().map(|_| "[voice note]".into()));

            self.signal
                .reply_quoted_target(
                    message,
                    quote.id,
                    author,
                    snippet.as_deref(),
                    body,
                )
                .await?;
        } else {
            self.signal.reply(message, body).await?;
        }
        Ok(())
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

    async fn transcribe_audio(&self, audio: &Attachment, bytes: &[u8]) -> Result<String, WhisperError> {
        let filename = VoiceHandler::attachment_filename(audio);
        let transcript = self
            .whisper
            .transcribe(bytes, &filename, &audio.content_type)
            .await?;
        Ok(VoiceHandler::format_transcript(
            transcript.trimmed_text(),
            &self.reply_prefix,
        ))
    }
}

#[async_trait]
impl CommandHandler for ManualTranscribeHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        message.text.trim() == "!transcribe"
    }

    fn handles_own_reply(&self) -> bool {
        true
    }

    fn label(&self) -> &'static str {
        "manual_transcribe"
    }

    #[instrument(skip(self, message), fields(source = %message.source, is_group = message.is_group))]
    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        let quote = match &message.quote {
            Some(q) => q,
            None => {
                let msg = "Reply to a voice message with: !transcribe";
                self.send_reply(message, None, msg).await?;
                return Ok(String::new());
            }
        };

        let audio = match &quote.audio_attachment {
            Some(a) => a,
            None => {
                let msg = "Quoted message has no voice attachment. Reply to a voice note.";
                self.send_reply(message, Some(quote), msg).await?;
                return Ok(String::new());
            }
        };

        if let Some(expected) = audio.size {
            if expected > self.max_attachment_bytes as i64 {
                let msg = "Voice note too long (max 5 min). Send a shorter clip.";
                self.send_reply(message, Some(quote), msg).await?;
                return Ok(String::new());
            }
        }

        let bytes = match self.signal.download_attachment(&audio.id).await {
            Ok(bytes) => bytes,
            Err(e) => {
                warn!("Failed to download quoted voice attachment {}: {}", audio.id, e);
                let msg = "Could not download voice note. Try again later.";
                self.send_reply(message, Some(quote), msg).await?;
                return Ok(String::new());
            }
        };

        if bytes.len() > self.max_attachment_bytes {
            let msg = "Voice note too long (max 5 min). Send a shorter clip.";
            self.send_reply(message, Some(quote), msg).await?;
            return Ok(String::new());
        }

        let body = match self.transcribe_audio(audio, &bytes).await {
            Ok(transcript) => {
                info!(
                    source = %message.source,
                    chars = transcript.len(),
                    "!transcribe completed"
                );
                transcript
            }
            Err(e) => {
                warn!("Whisper transcription failed: {}", e);
                Self::user_message_for_whisper_error(&e).to_string()
            }
        };

        self.send_reply(message, Some(quote), &body).await?;
        Ok(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use signal_client::BotMessage;

    #[test]
    fn matches_bare_command_only() {
        let handler = ManualTranscribeHandler::new(
            Arc::new(WhisperClient::new("http://localhost", std::time::Duration::from_secs(5)).unwrap()),
            Arc::new(SignalClient::new("http://localhost").unwrap()),
            "📝 Transcript:",
            5_000_000,
        );
        let mut msg = BotMessage {
            source: "+1".into(),
            text: "!transcribe".into(),
            timestamp: 0,
            message_timestamp: 0,
            is_group: false,
            group_id: None,
            receiving_account: "+2".into(),
            attachments: vec![],
            quote: None,
        };
        assert!(handler.matches(&msg));
        msg.text = "!transcribe-on".into();
        assert!(!handler.matches(&msg));
        msg.text = "!transcribe-off".into();
        assert!(!handler.matches(&msg));
    }
}
