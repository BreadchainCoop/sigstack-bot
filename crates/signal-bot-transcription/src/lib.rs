//! Voice transcription product handlers (Whisper pipeline).

mod fanout;
mod handlers;
mod manual_transcribe;
mod prefs;
mod transcribe;
mod transcribe_store;
mod voice;
mod voice_attachment_cache;

pub use fanout::{SharedTranscriptFanout, TranscriptFanout};
pub use handlers::build_voice_handlers;
pub use manual_transcribe::ManualTranscribeHandler;
pub use prefs::{SharedTranscribeGroupPrefs, TranscribeGroupPrefs};
pub use transcribe::TranscribeHandler;
pub use transcribe_store::TranscribeStore;
pub use voice::VoiceHandler;
pub use voice_attachment_cache::VoiceAttachmentCache;
