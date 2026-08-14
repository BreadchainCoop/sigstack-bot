//! Group-level transcription preference bridge (implemented by signal-bot prefs store).

use std::sync::Arc;

/// Per-group voice transcription toggle, owned by the shared preferences store.
pub trait TranscribeGroupPrefs: Send + Sync {
    fn is_transcribe_enabled(&self, group_id: &str) -> bool;
    fn set_transcribe_enabled(&self, group_id: &str, enabled: bool);
}

/// Helper so callers can pass `Arc<GroupPreferencesStore>` via a thin adapter.
pub type SharedTranscribeGroupPrefs = Arc<dyn TranscribeGroupPrefs>;
