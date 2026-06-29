//! Per-chat voice transcription preference (`!transcribe-on` / `!transcribe-off`).
//!
//! Group preferences are persisted via [`GroupPreferencesStore`]; DM toggles are ephemeral.

use crate::group_preferences_store::GroupPreferencesStore;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

/// DM-only in-memory transcription toggle (default: enabled).
pub struct TranscribeStore {
    dm_disabled: RwLock<HashSet<String>>,
    group_prefs: Option<Arc<GroupPreferencesStore>>,
}

impl TranscribeStore {
    pub fn new(group_prefs: Option<Arc<GroupPreferencesStore>>) -> Self {
        Self {
            dm_disabled: RwLock::new(HashSet::new()),
            group_prefs,
        }
    }

    pub fn is_enabled(&self, context_id: &str, is_group: bool) -> bool {
        if is_group {
            self.group_prefs
                .as_ref()
                .map(|store| store.is_transcribe_enabled(context_id))
                .unwrap_or(true)
        } else {
            !self.dm_disabled.read().unwrap().contains(context_id)
        }
    }

    pub fn set_enabled(&self, context_id: &str, enabled: bool, is_group: bool) {
        if is_group {
            if let Some(store) = &self.group_prefs {
                store.set_transcribe_enabled(context_id, enabled);
            }
            return;
        }

        let mut disabled = self.dm_disabled.write().unwrap();
        if enabled {
            disabled.remove(context_id);
        } else {
            disabled.insert(context_id.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dm_enabled_by_default() {
        let store = TranscribeStore::new(None);
        assert!(store.is_enabled("dm:+1234", false));
    }

    #[test]
    fn dm_toggle_off_and_on() {
        let store = TranscribeStore::new(None);
        let ctx = "dm:+1234";
        store.set_enabled(ctx, false, false);
        assert!(!store.is_enabled(ctx, false));
        store.set_enabled(ctx, true, false);
        assert!(store.is_enabled(ctx, false));
    }

    #[test]
    fn group_uses_preferences_store() {
        let prefs = GroupPreferencesStore::new_in_memory(0);
        let store = TranscribeStore::new(Some(prefs.clone()));
        store.set_enabled("group.x", false, true);
        assert!(!store.is_enabled("group.x", true));
    }
}
