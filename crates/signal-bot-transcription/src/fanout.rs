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

/// Fire-and-forget so NEAR chat does not block the handler after the transcript
/// quote-reply is sent. Callers must spawn only after that send succeeds.
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
pub(crate) struct RecordingFanout {
    pub events: std::sync::Mutex<Vec<&'static str>>,
    pub spoken: std::sync::Mutex<Vec<String>>,
    pub sources: std::sync::Mutex<Vec<String>>,
}

#[cfg(test)]
impl RecordingFanout {
    pub(crate) fn new() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            events: std::sync::Mutex::new(Vec::new()),
            spoken: std::sync::Mutex::new(Vec::new()),
            sources: std::sync::Mutex::new(Vec::new()),
        })
    }
}

#[cfg(test)]
#[async_trait]
impl TranscriptFanout for RecordingFanout {
    async fn fan_out_transcript(&self, original: &BotMessage, spoken_text: &str) {
        self.events.lock().unwrap().push("fanout");
        self.spoken.lock().unwrap().push(spoken_text.to_string());
        self.sources.lock().unwrap().push(original.source.clone());
    }
}

#[cfg(test)]
pub(crate) struct RecordSend(pub std::sync::Arc<RecordingFanout>);

#[cfg(test)]
impl wiremock::Respond for RecordSend {
    fn respond(&self, _request: &wiremock::Request) -> wiremock::ResponseTemplate {
        self.0.events.lock().unwrap().push("send");
        wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({}))
    }
}

#[cfg(test)]
pub(crate) async fn wait_for_fanout(rec: &RecordingFanout) {
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if rec.events.lock().unwrap().contains(&"fanout") {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("fan-out should run");
}

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
