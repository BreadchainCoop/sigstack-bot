//! In-process fan-out of transcripts into in-chat + Language Threads.

use crate::commands::{TranslateAllHandler, TranslateMeHandler};
use async_trait::async_trait;
use signal_bot_transcription::TranscriptFanout;
use signal_client::BotMessage;

pub struct SuiteTranscriptFanout {
    pub translate_all: Option<TranslateAllHandler>,
    pub translate_me: TranslateMeHandler,
}

#[async_trait]
impl TranscriptFanout for SuiteTranscriptFanout {
    async fn fan_out_transcript(&self, original: &BotMessage, spoken_text: &str) {
        if let Some(in_chat) = &self.translate_all {
            in_chat.fan_out_transcript(original, spoken_text).await;
        }
        self.translate_me
            .fan_out_transcript(original, spoken_text)
            .await;
    }
}
