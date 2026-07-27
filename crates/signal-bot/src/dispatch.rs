//! Match inbound messages to handlers and send replies.

use crate::bot_identity::BotIdentity;
use crate::commands::CommandHandler;
use signal_client::{BotMessage, SignalClient};
use tracing::{debug, error};

/// Result of attempting to dispatch one inbound message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchOutcome {
    /// A handler matched. `sent_reply` is false when the handler owns sending
    /// (`handles_own_reply`) or when the outbound send failed after logging.
    Matched {
        label: &'static str,
        sent_reply: bool,
        used_quote: bool,
    },
    /// No handler claimed the message.
    NoMatch,
}

/// Note bot identity, find the first matching handler, execute, and reply when needed.
pub async fn dispatch_message(
    handlers: &[Box<dyn CommandHandler>],
    signal: &SignalClient,
    bot_identity: &BotIdentity,
    message: &BotMessage,
) -> DispatchOutcome {
    bot_identity.note_inbound(message);

    let handler = handlers.iter().find(|h| h.matches(message));

    let Some(handler) = handler else {
        if message.is_voice_note() || !message.text.trim().is_empty() {
            debug!(
                source = %message.source,
                is_group = message.is_group,
                voice = message.is_voice_note(),
                "No handler matched message"
            );
        }
        return DispatchOutcome::NoMatch;
    };

    let quote_reply = handler.reply_with_quote();
    let own_reply = handler.handles_own_reply();
    let label = handler.label();
    debug!(
        handler = label,
        source = %message.source,
        is_group = message.is_group,
        voice = message.is_voice_note(),
        has_quote = message.quote.is_some(),
        own_reply,
        quote_reply,
        "Dispatching to handler"
    );

    match handler.execute(message).await {
        Ok(response) => {
            if own_reply {
                return DispatchOutcome::Matched {
                    label,
                    sent_reply: false,
                    used_quote: quote_reply,
                };
            }
            let send_result = if quote_reply {
                signal.reply_quoted(message, &response, None).await
            } else {
                signal.reply(message, &response).await
            };
            if let Err(e) = send_result {
                error!("Failed to send reply: {}", e);
                return DispatchOutcome::Matched {
                    label,
                    sent_reply: false,
                    used_quote: quote_reply,
                };
            }
            DispatchOutcome::Matched {
                label,
                sent_reply: true,
                used_quote: quote_reply,
            }
        }
        Err(e) => {
            error!("Handler error: {}", e);
            if own_reply {
                return DispatchOutcome::Matched {
                    label,
                    sent_reply: false,
                    used_quote: quote_reply,
                };
            }
            let fallback = "Sorry, something went wrong.";
            let send_result = if quote_reply {
                signal.reply_quoted(message, fallback, None).await
            } else {
                signal.reply(message, fallback).await
            };
            let sent_reply = send_result.is_ok();
            if let Err(send_err) = send_result {
                error!("Failed to send error fallback: {}", send_err);
            }
            DispatchOutcome::Matched {
                label,
                sent_reply,
                used_quote: quote_reply,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{AppError, AppResult};
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use wiremock::matchers::{body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    struct StubHandler {
        trigger: &'static str,
        label: &'static str,
        quote: bool,
        own_reply: bool,
        fail: bool,
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl CommandHandler for StubHandler {
        fn trigger(&self) -> Option<&str> {
            Some(self.trigger)
        }

        fn label(&self) -> &'static str {
            self.label
        }

        fn reply_with_quote(&self) -> bool {
            self.quote
        }

        fn handles_own_reply(&self) -> bool {
            self.own_reply
        }

        async fn execute(&self, _message: &BotMessage) -> AppResult<String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                Err(AppError::Config(anyhow::anyhow!("stub failure")))
            } else {
                Ok("stub-ok".into())
            }
        }
    }

    fn dm(text: &str) -> BotMessage {
        BotMessage {
            source: "+15550002222".into(),
            source_number: Some("+15550002222".into()),
            source_name: None,
            text: text.into(),
            timestamp: 1,
            message_timestamp: 42,
            is_group: false,
            group_id: None,
            group_name: None,
            receiving_account: "+15550001111".into(),
            attachments: vec![],
            quote: None,
        }
    }

    async fn signal_mock() -> (MockServer, SignalClient) {
        let server = MockServer::start().await;
        let client = SignalClient::new(server.uri()).unwrap();
        (server, client)
    }

    #[tokio::test]
    async fn no_match_when_no_handler_claims_message() {
        let (_server, signal) = signal_mock().await;
        let identity = BotIdentity::new();
        let handlers: Vec<Box<dyn CommandHandler>> = vec![Box::new(StubHandler {
            trigger: "!help",
            label: "help",
            quote: false,
            own_reply: false,
            fail: false,
            calls: Arc::new(AtomicUsize::new(0)),
        })];

        let outcome = dispatch_message(&handlers, &signal, &identity, &dm("hello")).await;
        assert_eq!(outcome, DispatchOutcome::NoMatch);
    }

    #[tokio::test]
    async fn plain_reply_sends_via_v2_send() {
        let (server, signal) = signal_mock().await;
        Mock::given(method("POST"))
            .and(path("/v2/send"))
            .and(body_partial_json(json!({
                "message": "stub-ok",
                "number": "+15550001111",
                "recipients": ["+15550002222"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount(&server)
            .await;

        let calls = Arc::new(AtomicUsize::new(0));
        let handlers: Vec<Box<dyn CommandHandler>> = vec![Box::new(StubHandler {
            trigger: "!help",
            label: "help",
            quote: false,
            own_reply: false,
            fail: false,
            calls: calls.clone(),
        })];
        let identity = BotIdentity::new();

        let outcome = dispatch_message(&handlers, &signal, &identity, &dm("!help")).await;
        assert_eq!(
            outcome,
            DispatchOutcome::Matched {
                label: "help",
                sent_reply: true,
                used_quote: false,
            }
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(identity.is_bot_message(&BotMessage {
            source: "+15550001111".into(),
            source_number: Some("+15550001111".into()),
            source_name: None,
            text: String::new(),
            timestamp: 0,
            message_timestamp: 0,
            is_group: false,
            group_id: None,
            group_name: None,
            receiving_account: "+15550001111".into(),
            attachments: vec![],
            quote: None,
        }));
    }

    #[tokio::test]
    async fn quote_reply_includes_quote_fields() {
        let (server, signal) = signal_mock().await;
        Mock::given(method("POST"))
            .and(path("/v2/send"))
            .and(body_partial_json(json!({
                "message": "stub-ok",
                "quote_timestamp": 42,
                "quote_author": "+15550002222"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount(&server)
            .await;

        let handlers: Vec<Box<dyn CommandHandler>> = vec![Box::new(StubHandler {
            trigger: "!voice",
            label: "voice",
            quote: true,
            own_reply: false,
            fail: false,
            calls: Arc::new(AtomicUsize::new(0)),
        })];
        let identity = BotIdentity::new();

        let outcome = dispatch_message(&handlers, &signal, &identity, &dm("!voice")).await;
        assert_eq!(
            outcome,
            DispatchOutcome::Matched {
                label: "voice",
                sent_reply: true,
                used_quote: true,
            }
        );
    }

    #[tokio::test]
    async fn own_reply_skips_outbound_send() {
        let (server, signal) = signal_mock().await;
        Mock::given(method("POST"))
            .and(path("/v2/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(0)
            .mount(&server)
            .await;

        let handlers: Vec<Box<dyn CommandHandler>> = vec![Box::new(StubHandler {
            trigger: "!me",
            label: "translate_me",
            quote: false,
            own_reply: true,
            fail: false,
            calls: Arc::new(AtomicUsize::new(0)),
        })];
        let identity = BotIdentity::new();

        let outcome = dispatch_message(&handlers, &signal, &identity, &dm("!me")).await;
        assert_eq!(
            outcome,
            DispatchOutcome::Matched {
                label: "translate_me",
                sent_reply: false,
                used_quote: false,
            }
        );
    }

    #[tokio::test]
    async fn handler_error_sends_fallback_message() {
        let (server, signal) = signal_mock().await;
        Mock::given(method("POST"))
            .and(path("/v2/send"))
            .and(body_partial_json(json!({
                "message": "Sorry, something went wrong."
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount(&server)
            .await;

        let handlers: Vec<Box<dyn CommandHandler>> = vec![Box::new(StubHandler {
            trigger: "!boom",
            label: "boom",
            quote: false,
            own_reply: false,
            fail: true,
            calls: Arc::new(AtomicUsize::new(0)),
        })];
        let identity = BotIdentity::new();

        let outcome = dispatch_message(&handlers, &signal, &identity, &dm("!boom")).await;
        assert_eq!(
            outcome,
            DispatchOutcome::Matched {
                label: "boom",
                sent_reply: true,
                used_quote: false,
            }
        );
    }

    #[tokio::test]
    async fn first_matching_handler_wins() {
        let (server, signal) = signal_mock().await;
        Mock::given(method("POST"))
            .and(path("/v2/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount(&server)
            .await;

        let first = Arc::new(AtomicUsize::new(0));
        let second = Arc::new(AtomicUsize::new(0));
        let handlers: Vec<Box<dyn CommandHandler>> = vec![
            Box::new(StubHandler {
                trigger: "!x",
                label: "first",
                quote: false,
                own_reply: false,
                fail: false,
                calls: first.clone(),
            }),
            Box::new(StubHandler {
                trigger: "!x",
                label: "second",
                quote: false,
                own_reply: false,
                fail: false,
                calls: second.clone(),
            }),
        ];
        let identity = BotIdentity::new();

        let outcome = dispatch_message(&handlers, &signal, &identity, &dm("!x")).await;
        assert_eq!(
            outcome,
            DispatchOutcome::Matched {
                label: "first",
                sent_reply: true,
                used_quote: false,
            }
        );
        assert_eq!(first.load(Ordering::SeqCst), 1);
        assert_eq!(second.load(Ordering::SeqCst), 0);
    }
}
