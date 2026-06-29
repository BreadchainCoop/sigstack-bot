//! Implicit voice note handler — transcribe via Whisper and quote-reply.

use crate::commands::translate_service::{
    format_voice_auto_translation, near_ai_translate, resolve_translate_all_voice_pair,
};
use crate::commands::CommandHandler;
use crate::group_translate_store::GroupTranslateStore;
use crate::transcribe_store::TranscribeStore;
use crate::error::AppResult;
use async_trait::async_trait;
use near_ai_client::NearAiClient;
use signal_client::{Attachment, BotMessage, SignalClient};
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};
use whisper_client::{WhisperClient, WhisperError};

#[cfg_attr(not(test), allow(dead_code))]
const DEFAULT_REPLY_PREFIX: &str = "📝 Transcript:";

pub struct VoiceHandler {
    whisper: Arc<WhisperClient>,
    signal: Arc<SignalClient>,
    reply_prefix: String,
    max_attachment_bytes: usize,
    group_translate: Option<Arc<GroupTranslateStore>>,
    near_ai: Option<Arc<NearAiClient>>,
    transcribe_store: Option<Arc<TranscribeStore>>,
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
            group_translate: None,
            near_ai: None,
            transcribe_store: None,
        }
    }

    pub fn with_transcribe_store(mut self, store: Arc<TranscribeStore>) -> Self {
        self.transcribe_store = Some(store);
        self
    }

    pub fn with_translate_all(
        mut self,
        store: Arc<GroupTranslateStore>,
        near_ai: Arc<NearAiClient>,
    ) -> Self {
        self.group_translate = Some(store);
        self.near_ai = Some(near_ai);
        self
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

    async fn translate_all_voice(
        &self,
        _message: &BotMessage,
        audio: &Attachment,
        bytes: &[u8],
        group_id: &str,
    ) -> AppResult<String> {
        let store = self
            .group_translate
            .as_ref()
            .expect("translate-all store required");
        let near_ai = self.near_ai.as_ref().expect("near_ai required for translate-all");
        let mode = store
            .get(group_id)
            .expect("translate-all mode must be active");

        let filename = Self::attachment_filename(audio);
        let content_type = &audio.content_type;

        let transcript = self
            .whisper
            .transcribe(bytes, &filename, content_type)
            .await?;
        let transcript_text = transcript.trimmed_text();
        let whisper_lang = transcript.language.as_deref();

        let (source, target) = match resolve_translate_all_voice_pair(&mode, whisper_lang, transcript_text)
        {
            Some(pair) => pair,
            None => {
                info!(
                    group_id,
                    whisper_lang,
                    "Voice transcript language not in translate-all pair — transcript only"
                );
                return Ok(Self::format_transcript(transcript_text, &self.reply_prefix));
            }
        };

        debug!(
            group_id,
            whisper_lang,
            source_lang = source.code,
            target_lang = target.code,
            whisper_translate = target.code == "en" && source.code != "en",
            "translate-all voice pipeline"
        );

        let translation = if target.code == "en" && source.code != "en" {
            match self
                .whisper
                .translate_to_english(bytes, &filename, content_type)
                .await
            {
                Ok(r) => r.trimmed_text().to_string(),
                Err(e) => {
                    warn!("Whisper translate failed, falling back to NEAR AI: {}", e);
                    near_ai_translate(near_ai, transcript_text, target).await?
                }
            }
        } else {
            near_ai_translate(near_ai, transcript_text, target).await?
        };

        Ok(format_voice_auto_translation(
            source,
            transcript_text,
            target,
            &translation,
        ))
    }
}

#[async_trait]
impl CommandHandler for VoiceHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        if !message.is_voice_note() {
            return false;
        }
        self.transcribe_store
            .as_ref()
            .is_none_or(|store| store.is_enabled(message.reply_target()))
    }

    fn reply_with_quote(&self) -> bool {
        true
    }

    fn label(&self) -> &'static str {
        "voice"
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

        let translate_all_active = message.group_id.as_ref().is_some_and(|gid| {
            self.group_translate
                .as_ref()
                .is_some_and(|s| s.is_active(gid))
        });

        if translate_all_active {
            let group_id = message.group_id.as_deref().unwrap();
            let store = self.group_translate.as_ref().unwrap();
            if store.allow_message(group_id) {
                match self.translate_all_voice(message, audio, &bytes, group_id).await {
                    Ok(response) => {
                        info!(
                            source = %message.source,
                            chars = response.len(),
                            "Voice note translated (translate-all)"
                        );
                        return Ok(response);
                    }
                    Err(e) => {
                        warn!("translate-all voice failed: {}", e);
                        return Ok("Could not transcribe voice note. Try again later.".into());
                    }
                }
            } else {
                warn!(group_id, "translate-all rate limited — transcript only");
            }
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
