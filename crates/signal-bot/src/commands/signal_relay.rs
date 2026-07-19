//! `!signal` — lets a Pacto user DM a Signal user through the bot.
//!
//! This is the mirror of `!pact`. Because it can deliver to arbitrary Signal
//! numbers, it is off by default and gated by an allowlist. The relayed message
//! is prefixed with the sender's Pacto npub so the Signal recipient knows who
//! it is from and can reply with `!pact <npub> <message>`.

use crate::commands::CommandHandler;
use crate::error::AppResult;
use async_trait::async_trait;
use signal_client::{BotMessage, SignalClient};
use std::sync::Arc;
use tracing::info;

pub struct SignalRelayHandler {
    signal: Arc<SignalClient>,
    /// The bot's own Signal number (the "from" for relayed messages).
    from_number: String,
    /// Permitted recipient numbers (E.164), or `["*"]` for any.
    allowlist: Vec<String>,
}

impl SignalRelayHandler {
    pub fn new(signal: Arc<SignalClient>, from_number: String, allowlist: Vec<String>) -> Self {
        Self {
            signal,
            from_number,
            allowlist,
        }
    }

    fn usage(&self) -> String {
        "**Message a Signal user**\n\n\
         - !signal <+number> <message> — DM a Signal user\n\n\
         They'll see your Pacto npub and can reply with !pact."
            .to_string()
    }

    fn is_allowed(&self, recipient: &str) -> bool {
        self.allowlist.iter().any(|a| a == "*" || a == recipient)
    }
}

/// A plausible E.164 number: `+` followed by 7–15 digits.
fn is_e164(s: &str) -> bool {
    let digits = s.strip_prefix('+').unwrap_or("");
    !digits.is_empty()
        && digits.len() >= 7
        && digits.len() <= 15
        && digits.chars().all(|c| c.is_ascii_digit())
}

#[async_trait]
impl CommandHandler for SignalRelayHandler {
    fn trigger(&self) -> Option<&str> {
        Some("!signal")
    }

    fn label(&self) -> &'static str {
        "signal-relay"
    }

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        let args = message
            .text
            .strip_prefix("!signal")
            .unwrap_or_default()
            .trim();

        if args.is_empty() || args == "help" {
            return Ok(self.usage());
        }

        let Some((recipient, body)) = args.split_once(char::is_whitespace) else {
            return Ok(self.usage());
        };
        let recipient = recipient.trim();
        let body = body.trim();

        if !is_e164(recipient) {
            return Ok(format!(
                "'{recipient}' is not a valid phone number. Use E.164, e.g. +14155551234."
            ));
        }
        if body.is_empty() {
            return Ok("Message is empty. Usage: !signal <+number> <message>".to_string());
        }
        if !self.is_allowed(recipient) {
            return Ok(
                "That Signal number isn't reachable from Pacto (not on the allowlist).".to_string(),
            );
        }

        // `message.source` is the sender's Pacto npub. Prefix it so the Signal
        // user knows who it's from and can reply via `!pact <npub>`.
        let relayed = format!("💬 Pacto DM from {}:\n\n{}", message.source, body);

        self.signal
            .send(&self.from_number, recipient, &relayed)
            .await?;

        info!(recipient = %recipient, "Relayed Pacto→Signal DM");
        Ok(format!(
            "✅ Delivered to {}. They can reply with !pact {}.",
            recipient, message.source
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn handler(allowlist: &[&str]) -> SignalRelayHandler {
        SignalRelayHandler::new(
            Arc::new(SignalClient::new("http://localhost").unwrap()),
            "+15550000000".to_string(),
            allowlist.iter().map(|s| s.to_string()).collect(),
        )
    }

    fn dm(text: &str) -> BotMessage {
        BotMessage {
            source: "npub1sender".into(),
            text: text.into(),
            timestamp: 0,
            message_timestamp: 0,
            is_group: false,
            group_id: None,
            receiving_account: "sigstack".into(),
            attachments: vec![],
            quote: None,
        }
    }

    #[test]
    fn e164_validation() {
        assert!(is_e164("+14155551234"));
        assert!(is_e164("+15550000000"));
        assert!(!is_e164("14155551234")); // missing +
        assert!(!is_e164("+abc")); // non-digits
        assert!(!is_e164("+123")); // too short
        assert!(!is_e164("+1234567890123456")); // too long
    }

    #[test]
    fn allowlist_gates_recipients() {
        let h = handler(&["+14155551234"]);
        assert!(h.is_allowed("+14155551234"));
        assert!(!h.is_allowed("+19998887777"));

        let open = handler(&["*"]);
        assert!(open.is_allowed("+19998887777"));

        let empty = handler(&[]);
        assert!(!empty.is_allowed("+14155551234")); // empty = deny all
    }

    #[tokio::test]
    async fn usage_and_rejections_do_not_send() {
        let h = handler(&["+14155551234"]);
        // No network is touched on any of these paths.
        assert!(h.execute(&dm("!signal")).await.unwrap().contains("Message a Signal user"));
        assert!(
            h.execute(&dm("!signal notaphone hi"))
                .await
                .unwrap()
                .contains("not a valid phone number")
        );
        assert!(
            h.execute(&dm("!signal +19998887777 hi"))
                .await
                .unwrap()
                .contains("allowlist")
        );
        assert!(
            h.execute(&dm("!signal +14155551234"))
                .await
                .unwrap()
                .contains("Message a Signal user") // no body → usage
        );
    }
}
