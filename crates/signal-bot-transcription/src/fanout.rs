//! After STT, fan transcripts into in-chat translation / Language Threads.
//!
//! One Signal number never receives its own group posts, so translation must
//! run in-process instead of waiting for a second bot to see the transcript.

use async_trait::async_trait;
use signal_client::BotMessage;

/// Optional hook invoked with the original voice message and spoken text.
#[async_trait]
pub trait TranscriptFanout: Send + Sync {
    async fn fan_out_transcript(&self, original: &BotMessage, spoken_text: &str);
}

/// Fire-and-forget so the transcript quote-reply is not delayed by NEAR chat.
pub fn spawn_fanout(fanout: Option<SharedTranscriptFanout>, original: &BotMessage, spoken: &str) {
    let Some(fanout) = fanout else {
        return;
    };
    if spoken.trim().is_empty() {
        return;
    }
    let original = original.clone();
    let spoken = spoken.to_string();
    tokio::spawn(async move {
        fanout.fan_out_transcript(&original, &spoken).await;
    });
}

/// Shared handle for voice / `!transcribe` handlers.
pub type SharedTranscriptFanout = std::sync::Arc<dyn TranscriptFanout>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct Rec(Mutex<Vec<String>>);

    #[async_trait]
    impl TranscriptFanout for Rec {
        async fn fan_out_transcript(&self, _original: &BotMessage, spoken_text: &str) {
            self.0.lock().unwrap().push(spoken_text.to_string());
        }
    }

    fn msg() -> BotMessage {
        BotMessage {
            source: "+alice".into(),
            source_number: Some("+alice".into()),
            source_name: None,
            text: String::new(),
            timestamp: 1,
            message_timestamp: 1,
            is_group: true,
            group_id: Some("g".into()),
            group_name: None,
            receiving_account: "+bot".into(),
            attachments: vec![],
            quote: None,
        }
    }

    #[test]
    fn spawn_fanout_skips_none_and_empty() {
        spawn_fanout(None, &msg(), "hello");
        let rec = Arc::new(Rec(Mutex::new(Vec::new())));
        let fanout: SharedTranscriptFanout = rec.clone();
        spawn_fanout(Some(fanout), &msg(), "   ");
        assert!(rec.0.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn spawn_fanout_invokes_hook() {
        let rec = Arc::new(Rec(Mutex::new(Vec::new())));
        let fanout: SharedTranscriptFanout = rec.clone();
        spawn_fanout(Some(fanout), &msg(), "hola");
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if !rec.0.lock().unwrap().is_empty() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("fan-out should run");
        assert_eq!(*rec.0.lock().unwrap(), vec!["hola".to_string()]);
    }
}
