//! Per-chat voice transcription preference (`!transcribe-on` / `!transcribe-off`).
//!
//! Group preferences go through [`TranscribeGroupPrefs`]; DM toggles are ephemeral.
//! Auto transcription defaults **off** until explicitly enabled.

use crate::prefs::SharedTranscribeGroupPrefs;
use std::collections::HashSet;
use std::sync::RwLock;

/// DM-only in-memory transcription toggle (default: disabled).
pub struct TranscribeStore {
    dm_enabled: RwLock<HashSet<String>>,
    group_prefs: Option<SharedTranscribeGroupPrefs>,
}

impl TranscribeStore {
    pub fn new(group_prefs: Option<SharedTranscribeGroupPrefs>) -> Self {
        Self {
            dm_enabled: RwLock::new(HashSet::new()),
            group_prefs,
        }
    }

    pub fn is_enabled(&self, context_id: &str, is_group: bool) -> bool {
        if is_group {
            self.group_prefs
                .as_ref()
                .map(|store| store.is_transcribe_enabled(context_id))
                .unwrap_or(false)
        } else {
            self.dm_enabled.read().unwrap().contains(context_id)
        }
    }

    pub fn set_enabled(&self, context_id: &str, enabled: bool, is_group: bool) {
        if is_group {
            if let Some(store) = &self.group_prefs {
                store.set_transcribe_enabled(context_id, enabled);
            }
            return;
        }

        let mut enabled_dms = self.dm_enabled.write().unwrap();
        if enabled {
            enabled_dms.insert(context_id.to_string());
        } else {
            enabled_dms.remove(context_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prefs::TranscribeGroupPrefs;
    use std::collections::HashMap;
    use std::sync::Arc;

    struct MemoryPrefs {
        enabled: RwLock<HashMap<String, bool>>,
    }

    impl TranscribeGroupPrefs for MemoryPrefs {
        fn is_transcribe_enabled(&self, group_id: &str) -> bool {
            self.enabled
                .read()
                .unwrap()
                .get(group_id)
                .copied()
                .unwrap_or(false)
        }

        fn set_transcribe_enabled(&self, group_id: &str, enabled: bool) {
            self.enabled
                .write()
                .unwrap()
                .insert(group_id.to_string(), enabled);
        }
    }

    #[test]
    fn dm_disabled_by_default() {
        let store = TranscribeStore::new(None);
        assert!(!store.is_enabled("dm:+1234", false));
    }

    #[test]
    fn dm_toggle_off_and_on() {
        let store = TranscribeStore::new(None);
        let ctx = "dm:+1234";
        store.set_enabled(ctx, true, false);
        assert!(store.is_enabled(ctx, false));
        store.set_enabled(ctx, false, false);
        assert!(!store.is_enabled(ctx, false));
    }

    #[test]
    fn group_uses_preferences_store() {
        let prefs: SharedTranscribeGroupPrefs = Arc::new(MemoryPrefs {
            enabled: RwLock::new(HashMap::new()),
        });
        let store = TranscribeStore::new(Some(prefs.clone()));
        assert!(!store.is_enabled("group.x", true));
        store.set_enabled("group.x", true, true);
        assert!(store.is_enabled("group.x", true));
    }
}
