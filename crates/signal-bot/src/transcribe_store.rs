//! Per-chat voice transcription preference (`!transcribe-on` / `!transcribe-off`).

use std::collections::HashSet;
use std::sync::RwLock;

/// In-memory per-context transcription toggle (default: enabled).
pub struct TranscribeStore {
    disabled: RwLock<HashSet<String>>,
}

impl TranscribeStore {
    pub fn new() -> Self {
        Self {
            disabled: RwLock::new(HashSet::new()),
        }
    }

    pub fn is_enabled(&self, context_id: &str) -> bool {
        !self.disabled.read().unwrap().contains(context_id)
    }

    pub fn set_enabled(&self, context_id: &str, enabled: bool) {
        let mut disabled = self.disabled.write().unwrap();
        if enabled {
            disabled.remove(context_id);
        } else {
            disabled.insert(context_id.to_string());
        }
    }
}

impl Default for TranscribeStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enabled_by_default() {
        let store = TranscribeStore::new();
        assert!(store.is_enabled("group.test"));
    }

    #[test]
    fn toggle_off_and_on() {
        let store = TranscribeStore::new();
        let ctx = "dm:+1234";
        store.set_enabled(ctx, false);
        assert!(!store.is_enabled(ctx));
        store.set_enabled(ctx, true);
        assert!(store.is_enabled(ctx));
    }
}
