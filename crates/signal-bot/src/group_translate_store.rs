//! In-memory per-group language pair for `!translate-all` mode.

use crate::commands::translate_lang::{resolve_language, Language};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// Active bidirectional translation pair for a Signal group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupTranslateMode {
    pub lang_a: String,
    pub lang_b: String,
}

impl GroupTranslateMode {
    pub fn new(lang_a: &Language, lang_b: &Language) -> Self {
        Self {
            lang_a: lang_a.code.to_string(),
            lang_b: lang_b.code.to_string(),
        }
    }

    /// Human-readable pair for confirmation messages.
    pub fn display_pair(&self) -> String {
        let a = resolve_language(&self.lang_a)
            .map(|l| format!("{} {}", l.flag, l.name))
            .unwrap_or_else(|| self.lang_a.clone());
        let b = resolve_language(&self.lang_b)
            .map(|l| format!("{} {}", l.flag, l.name))
            .unwrap_or_else(|| self.lang_b.clone());
        format!("{a} ↔ {b}")
    }

    /// If `source_code` matches one side of the pair, return the other language.
    pub fn target_for_source(&self, source_code: &str) -> Option<&'static Language> {
        let source = source_code.to_lowercase();
        if source == self.lang_a {
            resolve_language(&self.lang_b)
        } else if source == self.lang_b {
            resolve_language(&self.lang_a)
        } else {
            None
        }
    }

    pub fn source_language(&self, source_code: &str) -> Option<&'static Language> {
        resolve_language(source_code)
    }
}

/// Ephemeral store: lost on bot restart (by design).
pub struct GroupTranslateStore {
    modes: RwLock<HashMap<String, GroupTranslateMode>>,
    rate_limits: RwLock<HashMap<String, Vec<Instant>>>,
    max_per_minute: u32,
}

impl GroupTranslateStore {
    pub fn new(max_per_minute: u32) -> Self {
        Self {
            modes: RwLock::new(HashMap::new()),
            rate_limits: RwLock::new(HashMap::new()),
            max_per_minute,
        }
    }

    pub fn is_active(&self, group_id: &str) -> bool {
        self.modes.read().unwrap().contains_key(group_id)
    }

    pub fn get(&self, group_id: &str) -> Option<GroupTranslateMode> {
        self.modes.read().unwrap().get(group_id).cloned()
    }

    pub fn set(&self, group_id: String, mode: GroupTranslateMode) {
        self.modes.write().unwrap().insert(group_id, mode);
    }

    pub fn clear(&self, group_id: &str) -> bool {
        self.modes.write().unwrap().remove(group_id).is_some()
    }

    /// Returns false when the group exceeded `max_per_minute` in the rolling window.
    pub fn allow_message(&self, group_id: &str) -> bool {
        if self.max_per_minute == 0 {
            return true;
        }

        let mut limits = self.rate_limits.write().unwrap();
        let now = Instant::now();
        let window = Duration::from_secs(60);
        let entries = limits.entry(group_id.to_string()).or_default();
        entries.retain(|t| now.duration_since(*t) < window);

        if entries.len() >= self.max_per_minute as usize {
            return false;
        }

        entries.push(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_for_source_swaps_pair() {
        let mode = GroupTranslateMode::new(
            resolve_language("es").unwrap(),
            resolve_language("en").unwrap(),
        );
        assert_eq!(mode.target_for_source("es").unwrap().code, "en");
        assert_eq!(mode.target_for_source("en").unwrap().code, "es");
        assert!(mode.target_for_source("fr").is_none());
    }

    #[test]
    fn rate_limit_enforced_per_minute() {
        let store = GroupTranslateStore::new(2);
        let gid = "group.test";
        assert!(store.allow_message(gid));
        assert!(store.allow_message(gid));
        assert!(!store.allow_message(gid));
    }
}
