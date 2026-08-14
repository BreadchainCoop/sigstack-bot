//! `!transcribe` — quote-reply manual voice transcription via Whisper.

use crate::fanout::{spawn_fanout, SharedTranscriptFanout};
use crate::transcribe_store::TranscribeStore;
use crate::voice::VoiceHandler;
use crate::voice_attachment_cache::VoiceAttachmentCache;
use async_trait::async_trait;
use signal_bot_core::{AppResult, CommandHandler};
use signal_client::{Attachment, BotMessage, QuotedMessage, SignalClient};
use std::sync::Arc;
use tracing::{info, instrument, warn};
use whisper_client::{WhisperClient, WhisperError};

pub struct ManualTranscribeHandler {
    whisper: Arc<WhisperClient>,
    signal: Arc<SignalClient>,
    reply_prefix: String,
    max_attachment_bytes: usize,
    voice_cache: Arc<VoiceAttachmentCache>,
    transcribe_store: Arc<TranscribeStore>,
    fanout: Option<SharedTranscriptFanout>,
}

const AUTO_ALREADY_ON_MSG: &str = "Automatic transcription is already on. Voice notes are transcribed as they arrive — no need to !transcribe.";

impl ManualTranscribeHandler {
    pub fn new(
        whisper: Arc<WhisperClient>,
        signal: Arc<SignalClient>,
        reply_prefix: impl Into<String>,
        max_attachment_bytes: usize,
        voice_cache: Arc<VoiceAttachmentCache>,
        transcribe_store: Arc<TranscribeStore>,
    ) -> Self {
        Self {
            whisper,
            signal,
            reply_prefix: reply_prefix.into(),
            max_attachment_bytes,
            voice_cache,
            transcribe_store,
            fanout: None,
        }
    }

    pub fn with_fanout(mut self, fanout: Option<SharedTranscriptFanout>) -> Self {
        self.fanout = fanout;
        self
    }

    pub(crate) fn resolve_quoted_audio(
        quote: &QuotedMessage,
        chat_id: &str,
        cache: &VoiceAttachmentCache,
    ) -> Option<Attachment> {
        quote
            .audio_attachment
            .clone()
            .or_else(|| cache.lookup(chat_id, quote.id))
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
                .or_else(|| {
                    quote
                        .audio_attachment
                        .as_ref()
                        .map(|_| "[voice note]".into())
                });

            self.signal
                .reply_quoted_target(message, quote.id, author, snippet.as_deref(), body)
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

    async fn transcribe_audio(
        &self,
        audio: &Attachment,
        bytes: &[u8],
    ) -> Result<(String, String), WhisperError> {
        let filename = VoiceHandler::attachment_filename(audio);
        let transcript = self
            .whisper
            .transcribe(bytes, &filename, &audio.content_type)
            .await?;
        let spoken = transcript.trimmed_text().to_string();
        Ok((
            spoken.clone(),
            VoiceHandler::format_transcript(&spoken, &self.reply_prefix),
        ))
    }
}

fn speaker_msg_for_fanout(command: &BotMessage, quote: &QuotedMessage) -> BotMessage {
    let mut msg = command.clone();
    if let Some(author) = quote.author_number.as_deref() {
        if !author.is_empty() {
            msg.source = author.to_string();
            msg.source_number = Some(author.to_string());
        }
    }
    msg
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
        if self
            .transcribe_store
            .is_enabled(message.reply_target(), message.is_group)
        {
            self.send_reply(message, message.quote.as_ref(), AUTO_ALREADY_ON_MSG)
                .await?;
            return Ok(String::new());
        }

        let quote = match &message.quote {
            Some(q) => q,
            None => {
                let msg = "Reply to a voice message with: !transcribe";
                self.send_reply(message, None, msg).await?;
                return Ok(String::new());
            }
        };

        let chat_id = message.reply_target();
        let audio = match Self::resolve_quoted_audio(quote, chat_id, &self.voice_cache) {
            Some(a) => a,
            None => {
                warn!(
                    quote_id = quote.id,
                    chat_id,
                    has_quote_attachment = quote.audio_attachment.is_some(),
                    "Could not resolve quoted voice attachment"
                );
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
                warn!(
                    "Failed to download quoted voice attachment {}: {}",
                    audio.id, e
                );
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

        match self.transcribe_audio(&audio, &bytes).await {
            Ok((spoken, transcript)) => {
                info!(
                    source = %message.source,
                    chars = transcript.len(),
                    "!transcribe completed"
                );
                self.send_reply(message, Some(quote), &transcript).await?;
                spawn_fanout(
                    self.fanout.clone(),
                    &speaker_msg_for_fanout(message, quote),
                    &spoken,
                );
            }
            Err(e) => {
                warn!("Whisper transcription failed: {}", e);
                self.send_reply(
                    message,
                    Some(quote),
                    Self::user_message_for_whisper_error(&e),
                )
                .await?;
            }
        }

        Ok(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fanout::{wait_for_fanout, RecordSend, RecordingFanout, SharedTranscriptFanout};
    use signal_client::{BotMessage, QuotedMessage};

    fn sample_audio() -> signal_client::Attachment {
        signal_client::Attachment {
            content_type: "audio/ogg".into(),
            filename: None,
            id: "cached-voice-id".into(),
            size: Some(1024),
            upload_timestamp: Some(1_719_000_000_000),
        }
    }

    #[test]
    fn resolves_audio_from_cache_when_quote_has_no_attachment() {
        let cache = VoiceAttachmentCache::new(10);
        let audio = sample_audio();
        cache.remember("dm:+1", 1_719_000_000_000, audio);

        let quote = QuotedMessage {
            id: 1_719_000_000_000,
            author_number: Some("+1".into()),
            text: None,
            audio_attachment: None,
        };

        let resolved =
            ManualTranscribeHandler::resolve_quoted_audio(&quote, "dm:+1", &cache).unwrap();
        assert_eq!(resolved.id, "cached-voice-id");
    }

    fn empty_store() -> Arc<TranscribeStore> {
        Arc::new(TranscribeStore::new(None))
    }

    fn test_whisper(url: &str) -> Arc<WhisperClient> {
        Arc::new(
            WhisperClient::new(
                url,
                std::time::Duration::from_secs(5),
                "test-key",
                "openai/whisper-large-v3",
            )
            .unwrap(),
        )
    }

    #[test]
    fn matches_bare_command_only() {
        let handler = ManualTranscribeHandler::new(
            test_whisper("http://localhost"),
            Arc::new(SignalClient::new("http://localhost").unwrap()),
            "📝 Transcript:",
            5_000_000,
            VoiceAttachmentCache::new(10),
            empty_store(),
        );
        let mut msg = BotMessage {
            source: "+1".into(),
            source_number: None,
            source_name: None,
            text: "!transcribe".into(),
            timestamp: 0,
            message_timestamp: 0,
            is_group: false,
            group_id: None,
            group_name: None,
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

    fn quoted_transcribe_msg() -> BotMessage {
        BotMessage {
            source: "+15550002222".into(),
            source_number: Some("+15550002222".into()),
            source_name: None,
            text: "!transcribe".into(),
            timestamp: 2,
            message_timestamp: 2,
            is_group: false,
            group_id: None,
            group_name: None,
            receiving_account: "+15550001111".into(),
            attachments: vec![],
            quote: Some(QuotedMessage {
                id: 100,
                author_number: Some("+15550003333".into()),
                text: None,
                audio_attachment: Some(sample_audio()),
            }),
        }
    }

    fn typer_command() -> BotMessage {
        BotMessage {
            source: "+typer".into(),
            source_number: Some("+typer".into()),
            source_name: None,
            text: "!transcribe".into(),
            timestamp: 1,
            message_timestamp: 1,
            is_group: true,
            group_id: Some("gid".into()),
            group_name: None,
            receiving_account: "+bot".into(),
            attachments: vec![],
            quote: None,
        }
    }

    fn quote_with_author(author: Option<&str>) -> QuotedMessage {
        QuotedMessage {
            id: 1,
            author_number: author.map(str::to_string),
            text: None,
            audio_attachment: None,
        }
    }

    #[test]
    fn speaker_msg_for_fanout_uses_quote_author_when_present() {
        let out = speaker_msg_for_fanout(&typer_command(), &quote_with_author(Some("+speaker")));
        assert_eq!(out.source, "+speaker");
        assert_eq!(out.source_number.as_deref(), Some("+speaker"));
    }

    #[test]
    fn speaker_msg_for_fanout_keeps_typer_when_quote_has_no_author() {
        // Known gap: missing quote author attributes speech to the !transcribe typer.
        let out = speaker_msg_for_fanout(&typer_command(), &quote_with_author(None));
        assert_eq!(out.source, "+typer");
        assert_eq!(out.source_number.as_deref(), Some("+typer"));
    }

    #[test]
    fn speaker_msg_for_fanout_keeps_typer_when_quote_author_is_empty() {
        let out = speaker_msg_for_fanout(&typer_command(), &quote_with_author(Some("")));
        assert_eq!(out.source, "+typer");
        assert_eq!(out.source_number.as_deref(), Some("+typer"));
    }

    #[tokio::test]
    async fn execute_without_quote_sends_usage_hint() {
        use serde_json::json;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let signal_mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount(&signal_mock)
            .await;

        let rec = RecordingFanout::new();
        let fanout: SharedTranscriptFanout = rec.clone();
        let handler = ManualTranscribeHandler::new(
            test_whisper("http://127.0.0.1:9"),
            Arc::new(SignalClient::new(signal_mock.uri()).unwrap()),
            "📝 Transcript:",
            5_000_000,
            VoiceAttachmentCache::new(10),
            empty_store(),
        )
        .with_fanout(Some(fanout));

        let msg = BotMessage {
            source: "+15550002222".into(),
            source_number: Some("+15550002222".into()),
            source_name: None,
            text: "!transcribe".into(),
            timestamp: 1,
            message_timestamp: 1,
            is_group: false,
            group_id: None,
            group_name: None,
            receiving_account: "+15550001111".into(),
            attachments: vec![],
            quote: None,
        };
        let out = handler.execute(&msg).await.unwrap();
        assert!(out.is_empty());
        assert!(rec.spoken.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn execute_transcribes_quoted_audio() {
        use serde_json::json;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let signal_mock = MockServer::start().await;
        let whisper_mock = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/attachments/cached-voice-id"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"audio"))
            .mount(&signal_mock)
            .await;
        Mock::given(method("POST"))
            .and(path("/v2/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount(&signal_mock)
            .await;
        Mock::given(method("POST"))
            .and(path("/audio/transcriptions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "text": "quoted transcript",
                "language": "english"
            })))
            .mount(&whisper_mock)
            .await;

        let handler = ManualTranscribeHandler::new(
            test_whisper(&whisper_mock.uri()),
            Arc::new(SignalClient::new(signal_mock.uri()).unwrap()),
            "📝 Transcript:",
            5_000_000,
            VoiceAttachmentCache::new(10),
            empty_store(),
        );

        let out = handler.execute(&quoted_transcribe_msg()).await.unwrap();
        assert!(out.is_empty());
    }

    #[tokio::test]
    async fn execute_when_auto_on_replies_without_whisper() {
        use serde_json::json;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let signal_mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount(&signal_mock)
            .await;

        let store = empty_store();
        store.set_enabled("+15550002222", true, false);

        let rec = RecordingFanout::new();
        let fanout: SharedTranscriptFanout = rec.clone();
        let handler = ManualTranscribeHandler::new(
            test_whisper("http://127.0.0.1:9"),
            Arc::new(SignalClient::new(signal_mock.uri()).unwrap()),
            "📝 Transcript:",
            5_000_000,
            VoiceAttachmentCache::new(10),
            store,
        )
        .with_fanout(Some(fanout));

        let out = handler.execute(&quoted_transcribe_msg()).await.unwrap();
        assert!(out.is_empty());
        assert!(rec.spoken.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn execute_sends_transcript_before_fanout() {
        use serde_json::json;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let signal_mock = MockServer::start().await;
        let whisper_mock = MockServer::start().await;
        let rec = RecordingFanout::new();

        Mock::given(method("GET"))
            .and(path("/v1/attachments/cached-voice-id"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"audio"))
            .mount(&signal_mock)
            .await;
        Mock::given(method("POST"))
            .and(path("/v2/send"))
            .respond_with(RecordSend(rec.clone()))
            .expect(1)
            .mount(&signal_mock)
            .await;
        Mock::given(method("POST"))
            .and(path("/audio/transcriptions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "text": "quoted transcript",
                "language": "english"
            })))
            .mount(&whisper_mock)
            .await;

        let fanout: SharedTranscriptFanout = rec.clone();
        let handler = ManualTranscribeHandler::new(
            test_whisper(&whisper_mock.uri()),
            Arc::new(SignalClient::new(signal_mock.uri()).unwrap()),
            "📝 Transcript:",
            5_000_000,
            VoiceAttachmentCache::new(10),
            empty_store(),
        )
        .with_fanout(Some(fanout));

        handler.execute(&quoted_transcribe_msg()).await.unwrap();
        wait_for_fanout(&rec).await;
        assert_eq!(*rec.events.lock().unwrap(), vec!["send", "fanout"]);
        assert_eq!(
            *rec.spoken.lock().unwrap(),
            vec!["quoted transcript".to_string()]
        );
        assert_eq!(
            *rec.sources.lock().unwrap(),
            vec!["+15550003333".to_string()]
        );
    }

    #[tokio::test]
    async fn execute_skips_fanout_when_send_fails() {
        use serde_json::json;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let signal_mock = MockServer::start().await;
        let whisper_mock = MockServer::start().await;
        let rec = RecordingFanout::new();

        Mock::given(method("GET"))
            .and(path("/v1/attachments/cached-voice-id"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"audio"))
            .mount(&signal_mock)
            .await;
        Mock::given(method("POST"))
            .and(path("/v2/send"))
            .respond_with(ResponseTemplate::new(500).set_body_string("fail"))
            .mount(&signal_mock)
            .await;
        Mock::given(method("POST"))
            .and(path("/audio/transcriptions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "text": "quoted transcript",
                "language": "english"
            })))
            .mount(&whisper_mock)
            .await;

        let fanout: SharedTranscriptFanout = rec.clone();
        let handler = ManualTranscribeHandler::new(
            test_whisper(&whisper_mock.uri()),
            Arc::new(SignalClient::new(signal_mock.uri()).unwrap()),
            "📝 Transcript:",
            5_000_000,
            VoiceAttachmentCache::new(10),
            empty_store(),
        )
        .with_fanout(Some(fanout));

        assert!(handler.execute(&quoted_transcribe_msg()).await.is_err());
        assert!(rec.spoken.lock().unwrap().is_empty());
        assert!(rec.events.lock().unwrap().is_empty());
    }
}
