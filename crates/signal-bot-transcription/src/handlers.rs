//! Build the transcription product handler stack (voice / !transcribe*).

use crate::fanout::SharedTranscriptFanout;
use crate::manual_transcribe::ManualTranscribeHandler;
use crate::prefs::SharedTranscribeGroupPrefs;
use crate::transcribe::TranscribeHandler;
use crate::transcribe_store::TranscribeStore;
use crate::voice::VoiceHandler;
use crate::voice_attachment_cache::VoiceAttachmentCache;
use signal_bot_core::CommandHandler;
use signal_client::SignalClient;
use std::sync::Arc;
use whisper_client::WhisperClient;

/// Voice + `!transcribe` + `!transcribe-on/off` handlers.
pub fn build_voice_handlers(
    whisper: Arc<WhisperClient>,
    signal: Arc<SignalClient>,
    reply_prefix: impl Into<String>,
    max_attachment_bytes: usize,
    group_prefs: SharedTranscribeGroupPrefs,
    fanout: Option<SharedTranscriptFanout>,
) -> Vec<Box<dyn CommandHandler>> {
    let reply_prefix = reply_prefix.into();
    let transcribe_store = Arc::new(TranscribeStore::new(Some(group_prefs)));
    let voice_cache = VoiceAttachmentCache::with_default_capacity();

    vec![
        Box::new(
            VoiceHandler::new(
                whisper.clone(),
                signal.clone(),
                reply_prefix.clone(),
                max_attachment_bytes,
            )
            .with_transcribe_store(transcribe_store.clone())
            .with_voice_cache(voice_cache.clone())
            .with_fanout(fanout.clone()),
        ),
        Box::new(
            ManualTranscribeHandler::new(
                whisper,
                signal,
                reply_prefix,
                max_attachment_bytes,
                voice_cache,
                transcribe_store.clone(),
            )
            .with_fanout(fanout),
        ),
        Box::new(TranscribeHandler::new(transcribe_store, true)),
    ]
}
