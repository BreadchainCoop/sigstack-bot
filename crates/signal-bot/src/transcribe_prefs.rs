//! Adapter so [`GroupPreferencesStore`] implements transcription prefs trait.

use crate::group_preferences_store::GroupPreferencesStore;
use signal_bot_voice::TranscribeGroupPrefs;
use std::sync::Arc;

/// Thin wrapper so `set_transcribe_enabled` can use `Arc<GroupPreferencesStore>`.
pub struct GroupTranscribePrefs(pub Arc<GroupPreferencesStore>);

impl TranscribeGroupPrefs for GroupTranscribePrefs {
    fn is_transcribe_enabled(&self, group_id: &str) -> bool {
        self.0.is_transcribe_enabled(group_id)
    }

    fn set_transcribe_enabled(&self, group_id: &str, enabled: bool) {
        self.0.set_transcribe_enabled(group_id, enabled);
    }
}
