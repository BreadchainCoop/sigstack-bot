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
}

impl LanguageBridge {
    pub fn is_empty(&self) -> bool {
        self.sidecars.is_empty()
            && self.sidecar_internal.is_empty()
            && self.members.is_empty()
            && self.member_addresses.is_empty()
    }

    pub fn sidecar_send_id(&self, lang: &str) -> Option<&str> {
        self.sidecars.get(lang).map(String::as_str)
    }

    pub fn member_lang(&self, user: &str) -> Option<&str> {
        self.members.get(user).map(String::as_str)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GroupPreference {
    #[serde(default = "default_true")]
    transcribe_enabled: bool,
    #[serde(default)]
    translate: Option<GroupTranslateMode>,
    #[serde(default)]
    menu_language: MenuLanguage,
    /// Mutual-aid language sidecar bridge (replaces legacy per-user translate map).
    #[serde(default)]
    language_bridge: Option<LanguageBridge>,
}

impl Default for GroupPreference {
    fn default() -> Self {
        Self {
            transcribe_enabled: true,
            translate: None,
            menu_language: MenuLanguage::En,
            language_bridge: None,
        }
    }
}

impl GroupPreference {
    fn is_default(&self) -> bool {
        self.transcribe_enabled
            && self.translate.is_none()
            && self.menu_language == MenuLanguage::En
            && self.language_bridge.as_ref().is_none_or(LanguageBridge::is_empty)
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
            dstack: if persist {
                Some(dstack)
            } else {
                None
            },
            storage_path: if persist {
                Some(storage_path)
            } else {
                None
            },
            cached_key: RwLock::new(None),
            persist_lock: Mutex::new(()),
        });

        if persist {
            match store.load().await {
                Ok(count) => info!("Loaded group preferences for {count} groups"),
                Err(e) => warn!(
                    "Could not load group preferences (starting fresh): {e}"
                ),
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
            .is_none_or(|p| p.transcribe_enabled)
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

    // --- Auto-translate (per group) ---

    pub fn is_active(&self, group_id: &str) -> bool {
        self.groups
            .read()
            .unwrap()
            .get(group_id)
            .and_then(|p| p.translate.as_ref())
            .is_some()
    }

    pub fn get(&self, group_id: &str) -> Option<GroupTranslateMode> {
        self.groups
            .read()
            .unwrap()
            .get(group_id)
            .and_then(|p| p.translate.clone())
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

    // --- Language sidecar bridge (keyed by main group internal_id) ---

    pub fn get_bridge(&self, main_group_id: &str) -> Option<LanguageBridge> {
        self.groups
            .read()
            .unwrap()
            .get(main_group_id)
            .and_then(|p| p.language_bridge.clone())
            .filter(|b| !b.is_empty())
    }

    /// Resolve sidecar internal_id → (main_id, lang).
    pub fn lookup_sidecar(&self, sidecar_internal_id: &str) -> Option<(String, String)> {
        self.sidecar_index
            .read()
            .unwrap()
            .get(sidecar_internal_id)
            .cloned()
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
        {
            let mut groups = self.groups.write().unwrap();
            let entry = groups.entry(main_group_id.to_string()).or_default();
            let bridge = entry.language_bridge.get_or_insert_with(LanguageBridge::default);
            bridge.sidecars.insert(lang.to_string(), send_id);
            bridge
                .sidecar_internal
                .insert(lang.to_string(), internal_id);
        }
        self.rebuild_sidecar_index();
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
            let bridge = entry.language_bridge.get_or_insert_with(LanguageBridge::default);
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
            let Some(entry) = groups.get_mut(main_group_id) else {
                return None;
            };
            let Some(bridge) = entry.language_bridge.as_mut() else {
                return None;
            };
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

        debug!("Saved encrypted group preferences ({} bytes) to {path:?}", data.len());
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

fn default_true() -> bool {
    true
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
    fn transcribe_defaults_on() {
        let store = GroupPreferencesStore::new_in_memory(0);
        assert!(store.is_transcribe_enabled("group.new"));
    }

    #[test]
    fn transcribe_toggle_persists_in_memory() {
        let store = GroupPreferencesStore::new_in_memory(0);
        let gid = "group.abc";
        store.set_transcribe_enabled(gid, false);
        assert!(!store.is_transcribe_enabled(gid));
        store.set_transcribe_enabled(gid, true);
        assert!(store.is_transcribe_enabled(gid));
    }

    #[test]
    fn menu_language_defaults_english() {
        let store = GroupPreferencesStore::new_in_memory(0);
        assert_eq!(
            store.get_menu_language("group.new"),
            MenuLanguage::En
        );
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
        store.set_transcribe_enabled("group.two", false);
        store.set_menu_language("group.three", MenuLanguage::Es);
        store.persist_now().await.unwrap();

        let store2 =
            GroupPreferencesStore::with_test_key(DstackClient::new("/x"), path, key, 30).await;
        assert!(store2.is_active("group.one"));
        assert!(!store2.is_transcribe_enabled("group.two"));
        assert_eq!(
            store2.get_menu_language("group.three"),
            MenuLanguage::Es
        );
    }

    #[test]
    fn language_bridge_sidecar_and_members() {
        let store = GroupPreferencesStore::new_in_memory(0);
        let main = "main-internal";
        store.set_sidecar(
            main,
            "es",
            "group.es-send".into(),
            "es-internal".into(),
        );
        store.set_sidecar(
            main,
            "en",
            "group.en-send".into(),
            "en-internal".into(),
        );

        assert_eq!(
            store.lookup_sidecar("es-internal"),
            Some((main.into(), "es".into()))
        );
        assert_eq!(
            store.get_bridge(main).unwrap().sidecar_send_id("en"),
            Some("group.en-send")
        );

        assert!(store.set_bridge_member(main, "user-a", "es", Some("+1".into())).is_none());
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
        store.set_sidecar(
            "main-1",
            "es",
            "group.es".into(),
            "es-int".into(),
        );
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
    }
}
