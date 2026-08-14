//! Per-group bot preferences (transcription, auto-translate, menu language), TEE-encrypted at rest.

use crate::commands::translate_lang::{resolve_language, Language};
use crate::menu_language::MenuLanguage;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use dstack_client::DstackClient;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

const DATA_VERSION: u32 = 1;
const KEY_DERIVATION_PATH: &str = "signal-bot/group-preferences";
const NONCE_SIZE: usize = 12;

/// Active bidirectional translation pair for a Signal group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// Language sidecar bridge keyed under the **main** group `internal_id`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageBridge {
    /// lang code → sidecar send id (`group.…`)
    #[serde(default)]
    pub sidecars: HashMap<String, String>,
    /// lang code → sidecar internal_id (inbound match)
    #[serde(default)]
    pub sidecar_internal: HashMap<String, String>,
    /// user key (UUID or phone) → lang code
    #[serde(default)]
    pub members: HashMap<String, String>,
    /// user key → invite address used for Signal members[]
    #[serde(default)]
    pub member_addresses: HashMap<String, String>,
    /// Some = Bilingual Threads locked. None + non-empty sidecars = Language Threads.
    #[serde(default)]
    pub main_lang: Option<String>,
}

impl LanguageBridge {
    pub fn is_empty(&self) -> bool {
        self.sidecars.is_empty()
            && self.sidecar_internal.is_empty()
            && self.members.is_empty()
            && self.member_addresses.is_empty()
            && self.main_lang.is_none()
    }

    pub fn is_bilingual(&self) -> bool {
        self.main_lang.is_some()
    }

    /// Sidecar language when bilingual is locked (exactly one sidecar).
    pub fn bilingual_thread_lang(&self) -> Option<&str> {
        if !self.is_bilingual() {
            return None;
        }
        self.sidecars.keys().next().map(String::as_str)
    }

    pub fn sidecar_send_id(&self, lang: &str) -> Option<&str> {
        self.sidecars.get(lang).map(String::as_str)
    }

    pub fn member_lang(&self, user: &str) -> Option<&str> {
        self.members.get(user).map(String::as_str)
    }
}

/// Pending product switch after a refused enable (Threads ↔ in-chat mutual exclusion).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PendingSwitch {
    /// Apply `!translate-me-thread <lang>` after `!enable-threads`.
    EnableThreads {
        user: String,
        lang: String,
        #[serde(default)]
        address: Option<String>,
    },
    /// Apply `!translate-me-thread <main> <thread>` after `!enable-threads`.
    EnableBilingualThreads {
        user: String,
        main_lang: String,
        thread_lang: String,
        #[serde(default)]
        address: Option<String>,
    },
    /// Apply `!translate-all-on` after `!enable-in-chat`.
    EnableAllOn {
        user: String,
        lang_a: String,
        lang_b: String,
    },
    /// Apply `!translate-me-on` after `!enable-in-chat`.
    EnableMeOn {
        user: String,
        lang_a: String,
        lang_b: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GroupPreference {
    #[serde(default = "default_false")]
    transcribe_enabled: bool,
    #[serde(default)]
    translate: Option<GroupTranslateMode>,
    /// Per-user in-chat auto-translate pairs (`message.source` → pair).
    #[serde(default)]
    translate_members: HashMap<String, GroupTranslateMode>,
    #[serde(default)]
    menu_language: MenuLanguage,
    /// Mutual-aid language sidecar bridge (replaces legacy per-user translate map).
    #[serde(default)]
    language_bridge: Option<LanguageBridge>,
    #[serde(default)]
    pending_switch: Option<PendingSwitch>,
}

impl Default for GroupPreference {
    fn default() -> Self {
        Self {
            transcribe_enabled: false,
            translate: None,
            translate_members: HashMap::new(),
            menu_language: MenuLanguage::En,
            language_bridge: None,
            pending_switch: None,
        }
    }
}

impl GroupPreference {
    fn is_default(&self) -> bool {
        !self.transcribe_enabled
            && self.translate.is_none()
            && self.translate_members.is_empty()
            && self.menu_language == MenuLanguage::En
            && self
                .language_bridge
                .as_ref()
                .is_none_or(LanguageBridge::is_empty)
            && self.pending_switch.is_none()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct GroupPreferencesSnapshot {
    version: u32,
    groups: HashMap<String, GroupPreference>,
}

/// In-memory group preferences with optional TEE-encrypted persistence.
pub struct GroupPreferencesStore {
    groups: RwLock<HashMap<String, GroupPreference>>,
    /// sidecar internal_id → (main internal_id, lang code); rebuilt on load/mutate.
    sidecar_index: RwLock<HashMap<String, (String, String)>>,
    rate_limits: RwLock<HashMap<String, Vec<Instant>>>,
    max_per_minute: u32,
    dstack: Option<Arc<DstackClient>>,
    storage_path: Option<PathBuf>,
    cached_key: RwLock<Option<[u8; 32]>>,
    persist_lock: Mutex<()>,
}

impl GroupPreferencesStore {
    /// Memory-only store (lost on restart).
    pub fn new_in_memory(max_per_minute: u32) -> Arc<Self> {
        Arc::new(Self {
            groups: RwLock::new(HashMap::new()),
            sidecar_index: RwLock::new(HashMap::new()),
            rate_limits: RwLock::new(HashMap::new()),
            max_per_minute,
            dstack: None,
            storage_path: None,
            cached_key: RwLock::new(None),
            persist_lock: Mutex::new(()),
        })
    }

    /// Load from encrypted storage when `persist` is true; otherwise in-memory only.
    pub async fn open(
        dstack: Arc<DstackClient>,
        storage_path: PathBuf,
        persist: bool,
        max_per_minute: u32,
    ) -> Arc<Self> {
        let store = Arc::new(Self {
            groups: RwLock::new(HashMap::new()),
            sidecar_index: RwLock::new(HashMap::new()),
            rate_limits: RwLock::new(HashMap::new()),
            max_per_minute,
            dstack: if persist { Some(dstack) } else { None },
            storage_path: if persist { Some(storage_path) } else { None },
            cached_key: RwLock::new(None),
            persist_lock: Mutex::new(()),
        });

        if persist {
            match store.load().await {
                Ok(count) => info!("Loaded group preferences for {count} groups"),
                Err(e) => warn!("Could not load group preferences (starting fresh): {e}"),
            }
        }

        store
    }

    #[cfg(test)]
    pub async fn with_test_key(
        dstack: DstackClient,
        storage_path: PathBuf,
        key: [u8; 32],
        max_per_minute: u32,
    ) -> Arc<Self> {
        let store = Arc::new(Self {
            groups: RwLock::new(HashMap::new()),
            sidecar_index: RwLock::new(HashMap::new()),
            rate_limits: RwLock::new(HashMap::new()),
            max_per_minute,
            dstack: Some(Arc::new(dstack)),
            storage_path: Some(storage_path),
            cached_key: RwLock::new(Some(key)),
            persist_lock: Mutex::new(()),
        });
        let _ = store.load().await;
        store
    }

    fn rebuild_sidecar_index(&self) {
        let mut index = HashMap::new();
        for (main_id, pref) in self.groups.read().unwrap().iter() {
            if let Some(bridge) = &pref.language_bridge {
                for (lang, internal) in &bridge.sidecar_internal {
                    index.insert(internal.clone(), (main_id.clone(), lang.clone()));
                }
            }
        }
        *self.sidecar_index.write().unwrap() = index;
    }

    // --- Transcription (per group) ---

    pub fn is_transcribe_enabled(&self, group_id: &str) -> bool {
        self.groups
            .read()
            .unwrap()
            .get(group_id)
            .is_some_and(|p| p.transcribe_enabled)
    }

    pub fn set_transcribe_enabled(self: &Arc<Self>, group_id: &str, enabled: bool) {
        {
            let mut groups = self.groups.write().unwrap();
            let entry = groups.entry(group_id.to_string()).or_default();
            entry.transcribe_enabled = enabled;
            if entry.is_default() {
                groups.remove(group_id);
            }
        }
        self.schedule_persist();
    }

    // --- Menu language (per group) ---

    pub fn get_menu_language(&self, group_id: &str) -> MenuLanguage {
        self.groups
            .read()
            .unwrap()
            .get(group_id)
            .map(|p| p.menu_language)
            .unwrap_or_default()
    }

    pub fn set_menu_language(self: &Arc<Self>, group_id: &str, language: MenuLanguage) {
        {
            let mut groups = self.groups.write().unwrap();
            let entry = groups.entry(group_id.to_string()).or_default();
            entry.menu_language = language;
            if entry.is_default() {
                groups.remove(group_id);
            }
        }
        self.schedule_persist();
    }

    // --- Auto-translate (per group + per-user) ---

    pub fn is_active(&self, group_id: &str) -> bool {
        self.groups
            .read()
            .unwrap()
            .get(group_id)
            .and_then(|p| p.translate.as_ref())
            .is_some()
    }

    /// Group-wide or any personal in-chat auto-translate is configured.
    pub fn in_chat_auto_active(&self, group_id: &str) -> bool {
        self.groups
            .read()
            .unwrap()
            .get(group_id)
            .is_some_and(|p| p.translate.is_some() || !p.translate_members.is_empty())
    }

    /// Language Threads or Bilingual Threads bridge exists for this main group.
    pub fn threads_active(&self, main_group_id: &str) -> bool {
        self.get_bridge(main_group_id).is_some()
    }

    pub fn is_bilingual(&self, main_group_id: &str) -> bool {
        self.get_bridge(main_group_id)
            .is_some_and(|b| b.is_bilingual())
    }

    pub fn is_language_threads(&self, main_group_id: &str) -> bool {
        self.get_bridge(main_group_id)
            .is_some_and(|b| !b.is_bilingual())
    }

    pub fn get(&self, group_id: &str) -> Option<GroupTranslateMode> {
        self.groups
            .read()
            .unwrap()
            .get(group_id)
            .and_then(|p| p.translate.clone())
    }

    /// Resolve intercept pair: group-wide wins while set; otherwise personal for `user`.
    pub fn resolve_in_chat_mode(&self, group_id: &str, user: &str) -> Option<GroupTranslateMode> {
        let groups = self.groups.read().unwrap();
        let pref = groups.get(group_id)?;
        pref.translate
            .clone()
            .or_else(|| pref.translate_members.get(user).cloned())
    }

    pub fn set(self: &Arc<Self>, group_id: String, mode: GroupTranslateMode) {
        {
            let mut groups = self.groups.write().unwrap();
            let entry = groups.entry(group_id).or_default();
            entry.translate = Some(mode);
        }
        self.schedule_persist();
    }

    pub fn clear(self: &Arc<Self>, group_id: &str) -> bool {
        let had_translate = {
            let mut groups = self.groups.write().unwrap();
            let Some(entry) = groups.get_mut(group_id) else {
                return false;
            };
            let had = entry.translate.is_some();
            entry.translate = None;
            if entry.is_default() {
                groups.remove(group_id);
            }
            had
        };
        self.schedule_persist();
        had_translate
    }

    pub fn get_member_translate(&self, group_id: &str, user: &str) -> Option<GroupTranslateMode> {
        self.groups
            .read()
            .unwrap()
            .get(group_id)
            .and_then(|p| p.translate_members.get(user).cloned())
    }

    pub fn set_member_translate(
        self: &Arc<Self>,
        group_id: &str,
        user: &str,
        mode: GroupTranslateMode,
    ) {
        {
            let mut groups = self.groups.write().unwrap();
            let entry = groups.entry(group_id.to_string()).or_default();
            entry.translate_members.insert(user.to_string(), mode);
        }
        self.schedule_persist();
    }

    pub fn clear_member_translate(self: &Arc<Self>, group_id: &str, user: &str) -> bool {
        let cleared = {
            let mut groups = self.groups.write().unwrap();
            let Some(entry) = groups.get_mut(group_id) else {
                return false;
            };
            let had = entry.translate_members.remove(user).is_some();
            if entry.is_default() {
                groups.remove(group_id);
            }
            had
        };
        self.schedule_persist();
        cleared
    }

    /// Clear group-wide and all personal in-chat auto; returns whether anything was cleared.
    pub fn disable_in_chat(self: &Arc<Self>, group_id: &str) -> bool {
        let cleared = {
            let mut groups = self.groups.write().unwrap();
            let Some(entry) = groups.get_mut(group_id) else {
                return false;
            };
            let had = entry.translate.is_some() || !entry.translate_members.is_empty();
            entry.translate = None;
            entry.translate_members.clear();
            if entry.is_default() {
                groups.remove(group_id);
            }
            had
        };
        self.schedule_persist();
        cleared
    }

    /// Clear in-chat auto and consume pending switch without removing the group row.
    pub fn disable_in_chat_and_take_pending(
        self: &Arc<Self>,
        group_id: &str,
    ) -> (bool, Option<PendingSwitch>) {
        let result = {
            let mut groups = self.groups.write().unwrap();
            match groups.get_mut(group_id) {
                None => (false, None),
                Some(entry) => {
                    let had = entry.translate.is_some() || !entry.translate_members.is_empty();
                    entry.translate = None;
                    entry.translate_members.clear();
                    let pending = entry.pending_switch.take();
                    (had, pending)
                }
            }
        };
        self.schedule_persist();
        result
    }

    pub fn set_pending_switch(self: &Arc<Self>, group_id: &str, pending: PendingSwitch) {
        {
            let mut groups = self.groups.write().unwrap();
            let entry = groups.entry(group_id.to_string()).or_default();
            entry.pending_switch = Some(pending);
        }
        self.schedule_persist();
    }

    pub fn take_pending_switch(self: &Arc<Self>, group_id: &str) -> Option<PendingSwitch> {
        let pending = {
            let mut groups = self.groups.write().unwrap();
            let entry = groups.get_mut(group_id)?;
            let pending = entry.pending_switch.take();
            if entry.is_default() {
                groups.remove(group_id);
            }
            pending
        };
        self.schedule_persist();
        pending
    }

    pub fn get_pending_switch(&self, group_id: &str) -> Option<PendingSwitch> {
        self.groups
            .read()
            .unwrap()
            .get(group_id)
            .and_then(|p| p.pending_switch.clone())
    }

    // --- Language sidecar bridge (keyed by main group internal_id) ---

    pub fn get_bridge(&self, main_group_id: &str) -> Option<LanguageBridge> {
        self.groups
            .read()
            .unwrap()
            .get(main_group_id)
            .and_then(|p| p.language_bridge.clone())
            .filter(|b| !b.is_empty())
    }

    /// Remove and return the language bridge (for `!enable-in-chat` teardown).
    pub fn take_bridge(self: &Arc<Self>, main_group_id: &str) -> Option<LanguageBridge> {
        let bridge = {
            let mut groups = self.groups.write().unwrap();
            let entry = groups.get_mut(main_group_id)?;
            let bridge = entry.language_bridge.take().filter(|b| !b.is_empty());
            if entry.is_default() {
                groups.remove(main_group_id);
            }
            bridge
        };
        self.rebuild_sidecar_index();
        self.schedule_persist();
        bridge
    }

    /// Resolve sidecar internal_id → (main_id, lang).
    pub fn lookup_sidecar(&self, sidecar_internal_id: &str) -> Option<(String, String)> {
        self.sidecar_index
            .read()
            .unwrap()
            .get(sidecar_internal_id)
            .cloned()
    }

    /// Match inbound sidecar send id (`group.…`) when index only has internal ids.
    pub fn lookup_sidecar_by_send_id(&self, send_id: &str) -> Option<(String, String)> {
        for (main_id, pref) in self.groups.read().unwrap().iter() {
            if let Some(bridge) = &pref.language_bridge {
                for (lang, sid) in &bridge.sidecars {
                    if sid == send_id {
                        return Some((main_id.clone(), lang.clone()));
                    }
                }
            }
        }
        None
    }

    pub fn update_sidecar_internal(
        self: &Arc<Self>,
        main_group_id: &str,
        lang: &str,
        internal_id: &str,
    ) {
        let updated = {
            let mut groups = self.groups.write().unwrap();
            match groups.get_mut(main_group_id) {
                None => false,
                Some(entry) => match entry.language_bridge.as_mut() {
                    None => false,
                    Some(bridge) => {
                        if bridge.sidecar_internal.get(lang).map(String::as_str)
                            == Some(internal_id)
                        {
                            return;
                        }
                        bridge
                            .sidecar_internal
                            .insert(lang.to_string(), internal_id.to_string());
                        true
                    }
                },
            }
        };
        if updated {
            self.rebuild_sidecar_index();
            self.schedule_persist();
        }
    }

    /// Fix stored internal id using `list_groups` output; returns route when matched.
    pub fn reconcile_sidecar_internal_from_groups(
        self: &Arc<Self>,
        inbound_internal_id: &str,
        groups: &[signal_client::Group],
    ) -> Option<(String, String)> {
        let send_id = groups
            .iter()
            .find(|g| g.internal_id == inbound_internal_id)
            .map(|g| g.id.as_str())?;
        let mut matched: Option<(String, String)> = None;
        for (main_id, pref) in self.groups.read().unwrap().iter() {
            if let Some(bridge) = &pref.language_bridge {
                for (lang, sid) in &bridge.sidecars {
                    if sid == send_id {
                        matched = Some((main_id.clone(), lang.clone()));
                        break;
                    }
                }
            }
            if matched.is_some() {
                break;
            }
        }
        let (main_id, lang) = matched?;
        self.update_sidecar_internal(&main_id, &lang, inbound_internal_id);
        Some((main_id, lang))
    }

    pub fn member_lang(&self, main_group_id: &str, user: &str) -> Option<String> {
        self.get_bridge(main_group_id)
            .and_then(|b| b.members.get(user).cloned())
    }

    pub fn set_sidecar(
        self: &Arc<Self>,
        main_group_id: &str,
        lang: &str,
        send_id: String,
        internal_id: String,
    ) {
        self.insert_sidecar(main_group_id, lang, send_id, internal_id, None);
    }

    /// Register the one bilingual sidecar and lock `main_lang` in the same write.
    pub fn set_bilingual_sidecar(
        self: &Arc<Self>,
        main_group_id: &str,
        main_lang: &str,
        thread_lang: &str,
        send_id: String,
        internal_id: String,
    ) {
        self.insert_sidecar(
            main_group_id,
            thread_lang,
            send_id,
            internal_id,
            Some(main_lang),
        );
    }

    fn insert_sidecar(
        self: &Arc<Self>,
        main_group_id: &str,
        lang: &str,
        send_id: String,
        internal_id: String,
        bilingual_main: Option<&str>,
    ) {
        {
            let mut groups = self.groups.write().unwrap();
            let entry = groups.entry(main_group_id.to_string()).or_default();
            let bridge = entry
                .language_bridge
                .get_or_insert_with(LanguageBridge::default);
            bridge.sidecars.insert(lang.to_string(), send_id);
            bridge
                .sidecar_internal
                .insert(lang.to_string(), internal_id);
            if let Some(main_lang) = bilingual_main {
                bridge.main_lang = Some(main_lang.to_string());
            }
        }
        self.rebuild_sidecar_index();
        self.schedule_persist();
    }

    /// Lock Bilingual Threads on an existing sidecar (join path).
    pub fn set_bilingual_main_lang(self: &Arc<Self>, main_group_id: &str, lang: &str) {
        {
            let mut groups = self.groups.write().unwrap();
            let entry = groups.entry(main_group_id.to_string()).or_default();
            let bridge = entry
                .language_bridge
                .get_or_insert_with(LanguageBridge::default);
            bridge.main_lang = Some(lang.to_string());
        }
        self.schedule_persist();
    }

    /// Record user membership; returns previous lang if switching.
    pub fn set_bridge_member(
        self: &Arc<Self>,
        main_group_id: &str,
        user: &str,
        lang: &str,
        address: Option<String>,
    ) -> Option<String> {
        let previous = {
            let mut groups = self.groups.write().unwrap();
            let entry = groups.entry(main_group_id.to_string()).or_default();
            let bridge = entry
                .language_bridge
                .get_or_insert_with(LanguageBridge::default);
            let prev = bridge.members.insert(user.to_string(), lang.to_string());
            if let Some(addr) = address {
                bridge.member_addresses.insert(user.to_string(), addr);
            }
            prev
        };
        self.schedule_persist();
        previous
    }

    /// Remove member; returns (lang, address) if they were subscribed.
    pub fn clear_bridge_member(
        self: &Arc<Self>,
        main_group_id: &str,
        user: &str,
    ) -> Option<(String, Option<String>)> {
        let removed = {
            let mut groups = self.groups.write().unwrap();
            let entry = groups.get_mut(main_group_id)?;
            let bridge = entry.language_bridge.as_mut()?;
            let lang = bridge.members.remove(user)?;
            let address = bridge.member_addresses.remove(user);
            if bridge.is_empty() {
                entry.language_bridge = None;
            }
            if entry.is_default() {
                groups.remove(main_group_id);
            }
            Some((lang, address))
        };
        self.schedule_persist();
        removed
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

    fn schedule_persist(self: &Arc<Self>) {
        if self.storage_path.is_none() {
            return;
        }
        let store = Arc::clone(self);
        tokio::spawn(async move {
            if let Err(e) = store.persist().await {
                warn!("Failed to persist group preferences: {e}");
            }
        });
    }

    async fn derive_key(&self) -> Result<[u8; 32], String> {
        if let Some(key) = *self.cached_key.read().unwrap() {
            return Ok(key);
        }

        let dstack = self
            .dstack
            .as_ref()
            .ok_or_else(|| "persistence not configured".to_string())?;

        match dstack.derive_key(KEY_DERIVATION_PATH, None).await {
            Ok(key_bytes) => {
                if key_bytes.len() < 32 {
                    return Err(format!(
                        "Derived key too short: {} bytes (need 32)",
                        key_bytes.len()
                    ));
                }
                let mut key = [0u8; 32];
                key.copy_from_slice(&key_bytes[..32]);
                *self.cached_key.write().unwrap() = Some(key);
                info!("Using DeriveKey endpoint for group preferences encryption");
                return Ok(key);
            }
            Err(e) => {
                warn!("DeriveKey not available for group preferences, using AppInfo fallback: {e}");
            }
        }

        let app_info = dstack
            .get_app_info()
            .await
            .map_err(|e| format!("Failed to get AppInfo for key derivation: {e}"))?;

        let compose_hash = app_info.compose_hash.as_deref().unwrap_or("unknown");
        let app_id = app_info.app_id.as_deref().unwrap_or("unknown");

        let mut hasher = Sha256::new();
        hasher.update(compose_hash.as_bytes());
        hasher.update(app_id.as_bytes());
        hasher.update(KEY_DERIVATION_PATH.as_bytes());
        let hash = hasher.finalize();

        let mut key = [0u8; 32];
        key.copy_from_slice(&hash);
        *self.cached_key.write().unwrap() = Some(key);

        info!(
            "Using AppInfo-derived key for group preferences (compose_hash: {compose_hash}, app_id: {app_id})"
        );
        Ok(key)
    }

    fn snapshot(&self) -> GroupPreferencesSnapshot {
        GroupPreferencesSnapshot {
            version: DATA_VERSION,
            groups: self.groups.read().unwrap().clone(),
        }
    }

    async fn persist(&self) -> Result<(), String> {
        let _guard = self.persist_lock.lock().await;

        let path = self
            .storage_path
            .as_ref()
            .ok_or_else(|| "persistence not configured".to_string())?;

        let key = self.derive_key().await?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));

        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let plaintext = serde_json::to_vec(&self.snapshot())
            .map_err(|e| format!("serialize group preferences: {e}"))?;
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|e| format!("encrypt group preferences: {e}"))?;

        let mut data = nonce_bytes.to_vec();
        data.extend(ciphertext);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("create storage dir: {e}"))?;
        }

        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, &data)
            .await
            .map_err(|e| format!("write temp file: {e}"))?;
        fs::rename(&temp_path, path)
            .await
            .map_err(|e| format!("rename temp file: {e}"))?;

        debug!(
            "Saved encrypted group preferences ({} bytes) to {path:?}",
            data.len()
        );
        Ok(())
    }

    async fn load(&self) -> Result<usize, String> {
        let path = self
            .storage_path
            .as_ref()
            .ok_or_else(|| "persistence not configured".to_string())?;

        if !path.exists() {
            info!("Group preferences file not found at {path:?}, starting fresh");
            return Ok(0);
        }

        let key = self.derive_key().await?;
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
        let data = fs::read(path)
            .await
            .map_err(|e| format!("read group preferences: {e}"))?;

        if data.len() < NONCE_SIZE {
            return Err("group preferences file too short".into());
        }

        let nonce = Nonce::from_slice(&data[..NONCE_SIZE]);
        let ciphertext = &data[NONCE_SIZE..];
        let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|_| {
            "Failed to decrypt group preferences (TEE deployment may have changed)".to_string()
        })?;

        let snapshot: GroupPreferencesSnapshot = serde_json::from_slice(&plaintext)
            .map_err(|e| format!("parse group preferences: {e}"))?;

        if snapshot.version != DATA_VERSION {
            warn!(
                "Group preferences version {} != expected {DATA_VERSION}",
                snapshot.version
            );
        }

        let count = snapshot.groups.len();
        *self.groups.write().unwrap() = snapshot.groups;
        self.rebuild_sidecar_index();
        Ok(count)
    }

    #[cfg(test)]
    pub async fn persist_now(&self) -> Result<(), String> {
        self.persist().await
    }

    #[cfg(test)]
    pub async fn load_now(&self) -> Result<usize, String> {
        self.load().await
    }
}

fn default_false() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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
        let store = GroupPreferencesStore::new_in_memory(2);
        let gid = "group.test";
        assert!(store.allow_message(gid));
        assert!(store.allow_message(gid));
        assert!(!store.allow_message(gid));
    }

    #[test]
    fn personal_and_group_in_chat_helpers() {
        let store = GroupPreferencesStore::new_in_memory(0);
        let gid = "group.main";
        let mode = GroupTranslateMode::new(
            resolve_language("es").unwrap(),
            resolve_language("en").unwrap(),
        );

        assert!(!store.in_chat_auto_active(gid));
        store.set_member_translate(gid, "+alice", mode.clone());
        assert!(store.in_chat_auto_active(gid));
        assert!(!store.is_active(gid));
        assert_eq!(
            store.resolve_in_chat_mode(gid, "+alice").unwrap().lang_a,
            "es"
        );
        assert!(store.resolve_in_chat_mode(gid, "+bob").is_none());

        store.set(gid.into(), mode.clone());
        assert_eq!(
            store.resolve_in_chat_mode(gid, "+bob").unwrap().lang_a,
            "es"
        );
        // Group-wide wins over a stale personal pair (e.g. fa/en left from !translate-me-on).
        let fa_en = GroupTranslateMode::new(
            resolve_language("fa").unwrap(),
            resolve_language("en").unwrap(),
        );
        store.set_member_translate(gid, "+alice", fa_en);
        assert_eq!(
            store.resolve_in_chat_mode(gid, "+alice").unwrap().lang_a,
            "es"
        );
        assert_eq!(
            store.resolve_in_chat_mode(gid, "+alice").unwrap().lang_b,
            "en"
        );

        assert!(store.clear(gid));
        assert_eq!(
            store.resolve_in_chat_mode(gid, "+alice").unwrap().lang_a,
            "fa"
        );

        assert!(store.disable_in_chat(gid));
        assert!(!store.in_chat_auto_active(gid));
    }

    #[test]
    fn disable_in_chat_and_take_pending_keeps_group_row() {
        let store = GroupPreferencesStore::new_in_memory(0);
        let gid = "main";
        let mode = GroupTranslateMode::new(
            resolve_language("es").unwrap(),
            resolve_language("en").unwrap(),
        );
        store.set_member_translate(gid, "+alice", mode);
        store.set_pending_switch(
            gid,
            PendingSwitch::EnableThreads {
                user: "+alice".into(),
                lang: "es".into(),
                address: Some("+alice".into()),
            },
        );
        let (had, pending) = store.disable_in_chat_and_take_pending(gid);
        assert!(had);
        assert!(matches!(pending, Some(PendingSwitch::EnableThreads { .. })));
        assert!(!store.in_chat_auto_active(gid));
        assert!(store.groups.read().unwrap().contains_key(gid));
    }

    #[test]
    fn reconcile_sidecar_internal_from_groups() {
        let store = GroupPreferencesStore::new_in_memory(0);
        store.set_sidecar("main-internal", "es", "group.es".into(), "group.es".into());
        let groups = vec![signal_client::Group {
            name: "es".into(),
            id: "group.es".into(),
            internal_id: "es-internal".into(),
            members: vec![],
            pending_invites: vec![],
            pending_requests: vec![],
            admins: vec![],
        }];
        let route = store.reconcile_sidecar_internal_from_groups("es-internal", &groups);
        assert_eq!(route, Some(("main-internal".into(), "es".into())));
        assert_eq!(
            store.lookup_sidecar("es-internal"),
            Some(("main-internal".into(), "es".into()))
        );
    }

    #[test]
    fn pending_switch_and_take_bridge() {
        let store = GroupPreferencesStore::new_in_memory(0);
        let gid = "main";
        store.set_sidecar(gid, "es", "group.es".into(), "es-internal".into());
        assert!(store.threads_active(gid));
        store.set_pending_switch(
            gid,
            PendingSwitch::EnableAllOn {
                user: "+1".into(),
                lang_a: "es".into(),
                lang_b: "en".into(),
            },
        );
        let bridge = store.take_bridge(gid).unwrap();
        assert!(bridge.sidecars.contains_key("es"));
        assert!(!store.threads_active(gid));
        let pending = store.take_pending_switch(gid).unwrap();
        assert!(matches!(pending, PendingSwitch::EnableAllOn { .. }));
        assert!(store.take_pending_switch(gid).is_none());
    }

    #[test]
    fn transcribe_defaults_off() {
        let store = GroupPreferencesStore::new_in_memory(0);
        assert!(!store.is_transcribe_enabled("group.new"));
    }

    #[test]
    fn transcribe_toggle_persists_in_memory() {
        let store = GroupPreferencesStore::new_in_memory(0);
        let gid = "group.abc";
        store.set_transcribe_enabled(gid, true);
        assert!(store.is_transcribe_enabled(gid));
        store.set_transcribe_enabled(gid, false);
        assert!(!store.is_transcribe_enabled(gid));
    }

    #[test]
    fn menu_language_defaults_english() {
        let store = GroupPreferencesStore::new_in_memory(0);
        assert_eq!(store.get_menu_language("group.new"), MenuLanguage::En);
    }

    #[test]
    fn menu_language_toggle() {
        let store = GroupPreferencesStore::new_in_memory(0);
        let gid = "group.lang";
        store.set_menu_language(gid, MenuLanguage::Es);
        assert_eq!(store.get_menu_language(gid), MenuLanguage::Es);
        store.set_menu_language(gid, MenuLanguage::En);
        assert_eq!(store.get_menu_language(gid), MenuLanguage::En);
    }

    #[tokio::test]
    async fn encrypted_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("group_prefs.enc");
        let key = [7u8; 32];
        let dstack = DstackClient::new("/nonexistent/dstack.sock");

        let store = GroupPreferencesStore::with_test_key(dstack, path.clone(), key, 30).await;
        let mode = GroupTranslateMode::new(
            resolve_language("es").unwrap(),
            resolve_language("en").unwrap(),
        );
        store.set("group.one".into(), mode);
        store.set_transcribe_enabled("group.two", true);
        store.set_menu_language("group.three", MenuLanguage::Es);
        store.persist_now().await.unwrap();

        let store2 =
            GroupPreferencesStore::with_test_key(DstackClient::new("/x"), path, key, 30).await;
        assert!(store2.is_active("group.one"));
        assert!(store2.is_transcribe_enabled("group.two"));
        assert_eq!(store2.get_menu_language("group.three"), MenuLanguage::Es);
    }

    #[test]
    fn language_bridge_sidecar_and_members() {
        let store = GroupPreferencesStore::new_in_memory(0);
        let main = "main-internal";
        store.set_sidecar(main, "es", "group.es-send".into(), "es-internal".into());
        store.set_sidecar(main, "en", "group.en-send".into(), "en-internal".into());

        assert_eq!(
            store.lookup_sidecar("es-internal"),
            Some((main.into(), "es".into()))
        );
        assert_eq!(
            store.get_bridge(main).unwrap().sidecar_send_id("en"),
            Some("group.en-send")
        );

        assert!(store
            .set_bridge_member(main, "user-a", "es", Some("+1".into()))
            .is_none());
        assert_eq!(store.member_lang(main, "user-a").as_deref(), Some("es"));

        let prev = store.set_bridge_member(main, "user-a", "en", None);
        assert_eq!(prev.as_deref(), Some("es"));
        assert_eq!(store.member_lang(main, "user-a").as_deref(), Some("en"));

        let removed = store.clear_bridge_member(main, "user-a").unwrap();
        assert_eq!(removed.0, "en");
        assert!(store.member_lang(main, "user-a").is_none());
    }

    #[tokio::test]
    async fn language_bridge_encrypted_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bridge.enc");
        let key = [9u8; 32];
        let dstack = DstackClient::new("/nonexistent/dstack.sock");

        let store = GroupPreferencesStore::with_test_key(dstack, path.clone(), key, 30).await;
        store.set_sidecar("main-1", "es", "group.es".into(), "es-int".into());
        store.set_bridge_member("main-1", "uuid-1", "es", Some("+1555".into()));
        store.persist_now().await.unwrap();

        let store2 =
            GroupPreferencesStore::with_test_key(DstackClient::new("/x"), path, key, 30).await;
        let bridge = store2.get_bridge("main-1").unwrap();
        assert_eq!(bridge.sidecar_send_id("es"), Some("group.es"));
        assert_eq!(bridge.member_lang("uuid-1"), Some("es"));
        assert_eq!(
            store2.lookup_sidecar("es-int"),
            Some(("main-1".into(), "es".into()))
        );
        assert!(bridge.main_lang.is_none());
        assert!(!store2.is_bilingual("main-1"));
        assert!(store2.is_language_threads("main-1"));
    }

    #[test]
    fn language_bridge_main_lang_defaults_none_on_legacy_json() {
        let json = r#"{"sidecars":{"es":"group.es"},"sidecar_internal":{"es":"es-int"},"members":{},"member_addresses":{}}"#;
        let bridge: LanguageBridge = serde_json::from_str(json).unwrap();
        assert!(bridge.main_lang.is_none());
        assert!(!bridge.is_bilingual());
        assert!(!bridge.is_empty());
        assert_eq!(bridge.bilingual_thread_lang(), None);
    }

    #[test]
    fn bilingual_lock_is_not_empty_with_only_main_lang() {
        let bridge = LanguageBridge {
            main_lang: Some("es".into()),
            ..Default::default()
        };
        assert!(!bridge.is_empty());
        assert!(bridge.is_bilingual());
        assert_eq!(bridge.bilingual_thread_lang(), None);
    }

    #[test]
    fn bilingual_sidecar_locks_and_threads_active() {
        let store = GroupPreferencesStore::new_in_memory(0);
        let gid = "main-bi";
        store.set_bilingual_sidecar(gid, "es", "en", "group.en".into(), "en-internal".into());
        assert!(store.threads_active(gid));
        assert!(store.is_bilingual(gid));
        assert!(!store.is_language_threads(gid));
        let bridge = store.get_bridge(gid).unwrap();
        assert_eq!(bridge.main_lang.as_deref(), Some("es"));
        assert_eq!(bridge.bilingual_thread_lang(), Some("en"));
    }

    #[test]
    fn pending_enable_bilingual_threads_round_trip() {
        let pending = PendingSwitch::EnableBilingualThreads {
            user: "+1555".into(),
            main_lang: "es".into(),
            thread_lang: "en".into(),
            address: Some("+1555".into()),
        };
        let json = serde_json::to_string(&pending).unwrap();
        assert!(json.contains("enable_bilingual_threads"));
        let back: PendingSwitch = serde_json::from_str(&json).unwrap();
        assert_eq!(pending, back);

        let legacy: PendingSwitch =
            serde_json::from_str(r#"{"kind":"enable_threads","user":"+1","lang":"es"}"#).unwrap();
        assert!(matches!(
            legacy,
            PendingSwitch::EnableThreads { lang, .. } if lang == "es"
        ));
    }

    #[tokio::test]
    async fn bilingual_bridge_encrypted_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bilingual.enc");
        let key = [11u8; 32];
        let dstack = DstackClient::new("/nonexistent/dstack.sock");

        let store = GroupPreferencesStore::with_test_key(dstack, path.clone(), key, 30).await;
        store.set_bilingual_sidecar("main-1", "es", "en", "group.en".into(), "en-int".into());
        store.set_bridge_member("main-1", "uuid-1", "en", Some("+1555".into()));
        store.persist_now().await.unwrap();

        let store2 =
            GroupPreferencesStore::with_test_key(DstackClient::new("/x"), path, key, 30).await;
        let bridge = store2.get_bridge("main-1").unwrap();
        assert_eq!(bridge.main_lang.as_deref(), Some("es"));
        assert_eq!(bridge.sidecar_send_id("en"), Some("group.en"));
        assert!(store2.is_bilingual("main-1"));
        assert!(!store2.is_language_threads("main-1"));
    }
}
