//! `!translate` — quote-reply translation via NEAR AI.

use crate::commands::translate_lang::{resolve_language, Language};
use crate::commands::CommandHandler;
use crate::error::AppResult;
use async_trait::async_trait;
use near_ai_client::{Message, NearAiClient, Role};
use signal_client::{BotMessage, QuotedMessage, SignalClient};
use std::sync::Arc;
use tracing::{info, instrument, warn};

pub struct TranslateHandler {
    near_ai: Arc<NearAiClient>,
    signal: Arc<SignalClient>,
    transcript_prefix: String,
}

impl TranslateHandler {
    pub fn new(
        near_ai: Arc<NearAiClient>,
        signal: Arc<SignalClient>,
        transcript_prefix: impl Into<String>,
    ) -> Self {
        Self {
            near_ai,
            signal,
            transcript_prefix: transcript_prefix.into(),
        }
    }

    fn parse_lang_token(text: &str) -> Option<&str> {
        let rest = text.trim().strip_prefix("!translate")?.trim();
        if rest.is_empty() {
            return None;
        }
        Some(rest.split_whitespace().next().unwrap_or(""))
    }

    fn extract_quoted_text(quote: &QuotedMessage, transcript_prefix: &str) -> Option<String> {
        let raw = quote.text.as_ref()?.trim();
        if raw.is_empty() {
            return None;
        }

        let text = if let Some(rest) = raw.strip_prefix(transcript_prefix) {
            rest.trim_start_matches('\n').trim()
        } else {
            raw
        };

        if text.is_empty() {
            None
        } else {
            Some(text.to_string())
        }
    }

    fn quote_author(quote: &QuotedMessage) -> Option<&str> {
        quote.author_number.as_deref()
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
                .map(|t| truncate_snippet(t, 120));

            self.signal
                .reply_quoted_target(message, quote.id, author, snippet.as_deref(), body)
                .await?;
        } else {
            self.signal.reply(message, body).await?;
        }
        Ok(())
    }

    async fn translate_text(&self, source: &str, lang: &Language) -> Result<String, near_ai_client::NearAiError> {
        let prompt = format!(
            "Translate the following text to {}. Return only the translation, with no explanation or quotes.\n\n{}",
            lang.name, source
        );

        self.near_ai
            .chat(
                vec![
                    Message {
                        role: Role::System,
                        content: Some(
                            "You are a professional translator. Output only the translated text."
                                .into(),
                        ),
                        tool_calls: None,
                        tool_call_id: None,
                    },
                    Message {
                        role: Role::User,
                        content: Some(prompt),
                        tool_calls: None,
                        tool_call_id: None,
                    },
                ],
                Some(0.3),
                Some(1024),
            )
            .await
    }
}

fn truncate_snippet(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_len).collect();
        format!("{truncated}…")
    }
}

#[async_trait]
impl CommandHandler for TranslateHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        let text = message.text.trim();
        text.starts_with("!translate")
            && !text.starts_with("!translate-all")
            && !text.starts_with("!translate-off")
            && !text.starts_with("!translate-langs")
    }

    fn handles_own_reply(&self) -> bool {
        true
    }

    fn label(&self) -> &'static str {
        "translate"
    }

    #[instrument(skip(self, message), fields(source = %message.source, is_group = message.is_group))]
    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        let lang_token = match Self::parse_lang_token(&message.text) {
            Some(token) if !token.is_empty() => token,
            _ => {
                let msg = "Specify a language: !translate <language> (e.g. !translate es)";
                self.send_reply(message, None, msg).await?;
                return Ok(String::new());
            }
        };

        let lang = match resolve_language(lang_token) {
            Some(lang) => lang,
            None => {
                let msg = format!(
                    "Unknown language: {lang_token}. Use !translate-langs for supported codes."
                );
                self.send_reply(message, None, &msg).await?;
                return Ok(String::new());
            }
        };

        let quote = match &message.quote {
            Some(q) => q,
            None => {
                let msg =
                    "Reply to the message you want translated with: !translate <language>";
                self.send_reply(message, None, msg).await?;
                return Ok(String::new());
            }
        };

        let source = match Self::extract_quoted_text(quote, &self.transcript_prefix) {
            Some(text) => text,
            None => {
                let msg = "Could not read the quoted message text.";
                self.send_reply(message, Some(quote), msg).await?;
                return Ok(String::new());
            }
        };

        let body = match self.translate_text(&source, lang).await {
            Ok(translation) => {
                info!(
                    target_lang = lang.code,
                    source_chars = source.len(),
                    translation_chars = translation.len(),
                    "!translate completed"
                );
                format!("{} {}", lang.flag, translation.trim())
            }
            Err(e) => {
                warn!("NEAR AI translation failed: {}", e);
                "Could not translate. Try again later.".to_string()
            }
        };

        self.send_reply(message, Some(quote), &body).await?;
        Ok(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use signal_client::QuotedMessage;

    #[test]
    fn parse_lang_from_command() {
        assert_eq!(
            TranslateHandler::parse_lang_token("!translate es"),
            Some("es")
        );
        assert_eq!(
            TranslateHandler::parse_lang_token("!translate Spanish"),
            Some("Spanish")
        );
        assert_eq!(TranslateHandler::parse_lang_token("!translate"), None);
    }

    #[test]
    fn extract_text_strips_transcript_prefix() {
        let quote = QuotedMessage {
            id: 1,
            author_number: Some("+1".into()),
            text: Some("📝 Transcript:\nHola a todos".into()),
            audio_attachment: None,
        };
        let text =
            TranslateHandler::extract_quoted_text(&quote, "📝 Transcript:").unwrap();
        assert_eq!(text, "Hola a todos");
    }

    #[test]
    fn extract_text_without_prefix() {
        let quote = QuotedMessage {
            id: 1,
            author_number: Some("+1".into()),
            text: Some("Hello world".into()),
            audio_attachment: None,
        };
        let text = TranslateHandler::extract_quoted_text(&quote, "📝 Transcript:").unwrap();
        assert_eq!(text, "Hello world");
    }

    #[test]
    fn matches_excludes_translate_all_and_langs() {
        use signal_client::BotMessage;

        let handler = TranslateHandler::new(
            Arc::new(
                NearAiClient::new("key", "http://localhost", "model", std::time::Duration::from_secs(5))
                    .unwrap(),
            ),
            Arc::new(SignalClient::new("http://localhost").unwrap()),
            "📝 Transcript:",
        );

        let mut msg = BotMessage {
            source: "+1".into(),
            text: "!translate-all es en".into(),
            timestamp: 0,
            message_timestamp: 0,
            is_group: true,
            group_id: None,
            receiving_account: "+2".into(),
            attachments: vec![],
            quote: None,
        };
        assert!(!handler.matches(&msg));

        msg.text = "!translate-langs".into();
        assert!(!handler.matches(&msg));

        msg.text = "!translate es".into();
        assert!(handler.matches(&msg));
    }
}
