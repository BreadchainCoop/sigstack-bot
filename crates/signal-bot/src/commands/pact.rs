//! Pact command - sends an encrypted DM into Pacto via the pacto-bot-api daemon.

use crate::commands::CommandHandler;
use crate::error::AppResult;
use async_trait::async_trait;
use pacto_client::{PactoClient, PactoError};
use signal_client::BotMessage;
use std::sync::Arc;
use tracing::info;

pub struct PactHandler {
    pacto: Arc<PactoClient>,
    default_recipient: Option<String>,
}

impl PactHandler {
    pub fn new(pacto: Arc<PactoClient>, default_recipient: Option<String>) -> Self {
        Self {
            pacto,
            default_recipient,
        }
    }

    fn usage(&self) -> String {
        let default_line = match &self.default_recipient {
            Some(r) => format!("\nDefault recipient: {}", shorten_key(r)),
            None => String::new(),
        };
        format!(
            "**Pacto Messaging**\n\n\
             Send a message into Pacto:\n\
             - !pact <npub> <message> — DM a Pacto user\n\
             - !pact <message> — DM the default recipient{}\n\n\
             Messages are sent as bot `{}` via the pacto-bot-api daemon in this TEE.",
            default_line,
            self.pacto.bot_id(),
        )
    }
}

/// A token is treated as a recipient if it looks like a Nostr pubkey.
fn is_pubkey(token: &str) -> bool {
    (token.starts_with("npub1") && token.len() >= 60)
        || (token.len() == 64 && token.chars().all(|c| c.is_ascii_hexdigit()))
}

fn shorten_key(key: &str) -> String {
    if key.len() > 16 {
        format!("{}…{}", &key[..8], &key[key.len() - 4..])
    } else {
        key.to_string()
    }
}

#[async_trait]
impl CommandHandler for PactHandler {
    fn trigger(&self) -> Option<&str> {
        Some("!pact")
    }

    fn label(&self) -> &'static str {
        "pact"
    }

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        let args = message
            .text
            .strip_prefix("!pact")
            .unwrap_or_default()
            .trim();

        if args.is_empty() || args == "help" {
            return Ok(self.usage());
        }

        if args == "status" {
            return Ok(match self.pacto.version().await {
                Ok(v) => format!(
                    "Pacto daemon reachable (v{}, commit {}). Sending as bot `{}`.",
                    v.version,
                    v.commit.as_deref().unwrap_or("unknown"),
                    self.pacto.bot_id(),
                ),
                Err(e) => format!("Pacto daemon unreachable: {e}"),
            });
        }

        let (recipient, content) = match args.split_once(char::is_whitespace) {
            Some((first, rest)) if is_pubkey(first) => (first.to_string(), rest.trim()),
            _ => match &self.default_recipient {
                Some(default) => (default.clone(), args),
                None => {
                    return Ok(
                        "No recipient given and no default configured.\n\
                         Usage: !pact <npub> <message>"
                            .to_string(),
                    );
                }
            },
        };

        if content.is_empty() {
            return Ok("Message is empty. Usage: !pact <npub> <message>".to_string());
        }

        match self.pacto.send_dm(&recipient, content).await {
            Ok(event_id) => {
                info!(
                    recipient = %shorten_key(&recipient),
                    event_id = %event_id,
                    "Sent Pacto DM"
                );
                Ok(format!(
                    "✅ Sent to {} on Pacto (event {})",
                    shorten_key(&recipient),
                    shorten_key(&event_id),
                ))
            }
            Err(PactoError::SocketNotFound(_)) => Ok(
                "Pacto messaging is not available: the pacto-bot-api daemon is not running."
                    .to_string(),
            ),
            Err(PactoError::Rpc { message, .. }) => {
                Ok(format!("Pacto daemon rejected the message: {message}"))
            }
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pubkey_detection() {
        assert!(is_pubkey(
            "npub1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq"
        ));
        assert!(is_pubkey(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        ));
        assert!(!is_pubkey("hello"));
        assert!(!is_pubkey("npub1short"));
        // 63 hex chars is not a key
        assert!(!is_pubkey(&"a".repeat(63)));
    }

    #[test]
    fn shorten_key_truncates_long_keys() {
        assert_eq!(
            shorten_key("aaaaaaaabbbbbbbbccccccccdddd"),
            "aaaaaaaa…dddd"
        );
        assert_eq!(shorten_key("short"), "short");
    }
}
