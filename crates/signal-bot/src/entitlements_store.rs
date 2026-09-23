//! Encrypted entitlement records (paid / alpha plans), separate from feature prefs.
//!
//! # Plan composition (alpha vs paid)
//!
//! Alpha `bundle-all-alpha` and paid all-access packs grant full product coverage while
//! `active`/`past_due` and unexpired. A **paid** (`source: stripe`) entitlement that
//! overlaps a feature **replaces** alpha for that feature only; non-overlapping alpha
//! coverage remains. Paid packs cap how many Signal groups the subscriber may enable
//! via `!enable-sigstack`; alpha is uncapped. Stripe catalog SKUs must match the site
//! offer ids in `site/src/lib/content/en.ts` (plus `bundle-all-alpha`).

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use chrono::{DateTime, Utc};
use dstack_client::DstackClient;
use fs2::FileExt;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use tokio::fs;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// How often the bot refreshes entitlements from disk when a peer (commerce) may have written.
pub const ENTITLEMENTS_RELOAD_INTERVAL: std::time::Duration = std::time::Duration::from_secs(15);

const DATA_VERSION: u32 = 1;
const KEY_DERIVATION_PATH: &str = "signal-bot/entitlements";
const NONCE_SIZE: usize = 12;

/// Full product access (threads + in-chat + transcription), individual and group axes.
const ALL_ACCESS_GRANTS: &[FeatureGrant] = &[
    FeatureGrant::ThreadsIndividual,
    FeatureGrant::InChatMe,
    FeatureGrant::TranscriptionIndividual,
    FeatureGrant::ThreadsGroup,
    FeatureGrant::InChatAll,
];

/// Plan SKU strings aligned with `site/src/lib/content/en.ts` offer `id`s, plus alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlanSku {
    /// Paid: all-access for all members in up to 3 enabled groups.
    #[serde(rename = "all-access-3")]
    AllAccess3,
    /// Paid: all-access for all members in up to 10 enabled groups.
    #[serde(rename = "all-access-10")]
    AllAccess10,
    /// Commerce alpha: full all-access for a limited window (`source: alpha`), uncapped groups.
    BundleAllAlpha,
}

impl PlanSku {
    /// Feature grants implied by this SKU (before alpha/paid composition).
    pub fn grants(self) -> &'static [FeatureGrant] {
        match self {
            Self::AllAccess3 | Self::AllAccess10 | Self::BundleAllAlpha => ALL_ACCESS_GRANTS,
        }
    }

    /// Whether this SKU may enable a Signal group (`!enable-sigstack`).
    pub fn is_group_claimable(self) -> bool {
        matches!(
            self,
            Self::AllAccess3 | Self::AllAccess10 | Self::BundleAllAlpha
        )
    }

    /// Max groups the subscriber may enable; `None` = uncapped (alpha).
    pub fn max_groups(self) -> Option<usize> {
        match self {
            Self::AllAccess3 => Some(3),
            Self::AllAccess10 => Some(10),
            Self::BundleAllAlpha => None,
        }
    }
}

/// Billable feature axes used by future command / inference gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FeatureGrant {
    ThreadsIndividual,
    ThreadsGroup,
    InChatMe,
    InChatAll,
    TranscriptionIndividual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntitlementSource {
    Stripe,
    Alpha,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntitlementStatus {
    Active,
    PastDue,
    Expired,
    Canceled,
}

/// One entitlement row (individual pending/linked, and/or claimed to a group).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntitlementRecord {
    pub id: String,
    pub plan_sku: PlanSku,
    pub source: EntitlementSource,
    pub status: EntitlementStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stripe_customer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stripe_subscription_id: Option<String>,
    /// Signal user UUID preferred; phone E.164 when UUID unavailable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_token: Option<String>,
    /// Legacy single-group claim field; prefer `enabled_group_ids` for multi-group packs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claimed_group_id: Option<String>,
    /// Groups enabled via `!enable-sigstack` (alpha uncapped; paid packs respect `max_groups`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enabled_group_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl EntitlementRecord {
    pub fn is_granting_at(&self, now: DateTime<Utc>) -> bool {
        match self.status {
            EntitlementStatus::Active | EntitlementStatus::PastDue => {}
            EntitlementStatus::Expired | EntitlementStatus::Canceled => return false,
        }
        if let Some(expires) = self.expires_at {
            if expires <= now {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct EntitlementsSnapshot {
    version: u32,
    /// Owner key (Signal UUID or phone) → records.
    #[serde(default)]
    individuals: HashMap<String, Vec<EntitlementRecord>>,
    /// Group `internal_id` → claimed group-scoped records.
    #[serde(default)]
    groups: HashMap<String, Vec<EntitlementRecord>>,
    /// Pending checkout / alpha redeem awaiting `!link`.
    #[serde(default)]
    pending_by_token: HashMap<String, EntitlementRecord>,
}

fn appinfo_fallback_key(app_id: &str, compose_hash: Option<&str>) -> [u8; 32] {
    let mut hasher = Sha256::new();
    if let Some(compose_hash) = compose_hash {
        hasher.update(compose_hash.as_bytes());
    }
    hasher.update(app_id.as_bytes());
    hasher.update(KEY_DERIVATION_PATH.as_bytes());
    let hash = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash);
    key
}

fn decrypt_entitlements_blob(data: &[u8], key: &[u8; 32]) -> Result<EntitlementsSnapshot, String> {
    if data.len() < NONCE_SIZE {
        return Err("entitlements file too short".into());
    }
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(&data[..NONCE_SIZE]);
    let ciphertext = &data[NONCE_SIZE..];
    let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|_| {
        "Failed to decrypt entitlements (TEE deployment may have changed)".to_string()
    })?;
    serde_json::from_slice(&plaintext).map_err(|e| format!("parse entitlements: {e}"))
}

fn encrypt_entitlements_blob(
    snapshot: &EntitlementsSnapshot,
    key: &[u8; 32],
) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let plaintext =
        serde_json::to_vec(snapshot).map_err(|e| format!("serialize entitlements: {e}"))?;
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_ref())
        .map_err(|e| format!("encrypt entitlements: {e}"))?;
    let mut data = nonce_bytes.to_vec();
    data.extend(ciphertext);
    Ok(data)
}

fn push_unique_key(out: &mut Vec<(String, [u8; 32])>, label: String, key: [u8; 32]) {
    if out.iter().any(|(_, existing)| existing == &key) {
        return;
    }
    out.push((label, key));
}

fn new_record_id() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Temporary reusable friend code for alpha (each linker gets their own 90-day grant).
pub const REUSABLE_ALPHA_CODE: &str = "bread";

/// True when `code` matches the reusable friend alpha code (trim + case-insensitive).
pub fn is_reusable_alpha_code(code: &str) -> bool {
    code.trim().eq_ignore_ascii_case(REUSABLE_ALPHA_CODE)
}

/// Error from [`EntitlementsStore::redeem_reusable_alpha`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedeemReuseError {
    AlreadyActive,
    EmptyOwner,
}

impl std::fmt::Display for RedeemReuseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyActive => write!(f, "owner already has an active alpha entitlement"),
            Self::EmptyOwner => write!(f, "owner_uuid must be non-empty"),
        }
    }
}

/// Resolve effective feature → winning source after alpha/paid composition.
///
/// Paid (`stripe`) wins on overlap; alpha covers remaining bundle features.
pub fn compose_effective_grants(
    individual: &[EntitlementRecord],
    group: &[EntitlementRecord],
    now: DateTime<Utc>,
) -> HashMap<FeatureGrant, EntitlementSource> {
    let mut paid: HashSet<FeatureGrant> = HashSet::new();
    let mut alpha: HashSet<FeatureGrant> = HashSet::new();

    for record in individual.iter().chain(group.iter()) {
        if !record.is_granting_at(now) {
            continue;
        }
        let target = match record.source {
            EntitlementSource::Stripe => &mut paid,
            EntitlementSource::Alpha => &mut alpha,
        };
        for grant in record.plan_sku.grants() {
            target.insert(*grant);
        }
    }

    let mut out = HashMap::new();
    for grant in paid {
        out.insert(grant, EntitlementSource::Stripe);
    }
    for grant in alpha {
        out.entry(grant).or_insert(EntitlementSource::Alpha);
    }
    out
}

/// In-memory entitlements with optional TEE-encrypted persistence.
pub struct EntitlementsStore {
    individuals: RwLock<HashMap<String, Vec<EntitlementRecord>>>,
    groups: RwLock<HashMap<String, Vec<EntitlementRecord>>>,
    pending_by_token: RwLock<HashMap<String, EntitlementRecord>>,
    dstack: Option<Arc<DstackClient>>,
    storage_path: Option<PathBuf>,
    cached_key: RwLock<Option<[u8; 32]>>,
    persist_lock: Mutex<()>,
    /// File mtime observed at last successful load or persist (cross-process staleness).
    loaded_mtime: RwLock<Option<SystemTime>>,
    /// When true, [`Self::schedule_persist`] is a no-op (caller will persist under disk lock).
    defer_persist: AtomicBool,
    legacy_compose_hashes: Vec<String>,
}

impl EntitlementsStore {
    /// Memory-only store (lost on restart).
    pub fn new_in_memory() -> Arc<Self> {
        Arc::new(Self {
            individuals: RwLock::new(HashMap::new()),
            groups: RwLock::new(HashMap::new()),
            pending_by_token: RwLock::new(HashMap::new()),
            dstack: None,
            storage_path: None,
            cached_key: RwLock::new(None),
            persist_lock: Mutex::new(()),
            loaded_mtime: RwLock::new(None),
            defer_persist: AtomicBool::new(false),
            legacy_compose_hashes: Vec::new(),
        })
    }

    /// Load from encrypted storage when `persist` is true; otherwise in-memory only.
    pub async fn open(
        dstack: Arc<DstackClient>,
        storage_path: PathBuf,
        persist: bool,
        legacy_compose_hashes: Vec<String>,
    ) -> Arc<Self> {
        let store = Arc::new(Self {
            individuals: RwLock::new(HashMap::new()),
            groups: RwLock::new(HashMap::new()),
            pending_by_token: RwLock::new(HashMap::new()),
            dstack: if persist { Some(dstack) } else { None },
            storage_path: if persist { Some(storage_path) } else { None },
            cached_key: RwLock::new(None),
            persist_lock: Mutex::new(()),
            loaded_mtime: RwLock::new(None),
            defer_persist: AtomicBool::new(false),
            legacy_compose_hashes,
        });

        if persist {
            match store.load().await {
                Ok(count) => info!("Loaded {count} entitlement records"),
                Err(e) => warn!("Could not load entitlements (starting fresh): {e}"),
            }
        }

        store
    }

    #[cfg(test)]
    pub async fn with_test_key(
        dstack: DstackClient,
        storage_path: PathBuf,
        key: [u8; 32],
    ) -> Arc<Self> {
        let store = Arc::new(Self {
            individuals: RwLock::new(HashMap::new()),
            groups: RwLock::new(HashMap::new()),
            pending_by_token: RwLock::new(HashMap::new()),
            dstack: Some(Arc::new(dstack)),
            storage_path: Some(storage_path),
            cached_key: RwLock::new(Some(key)),
            persist_lock: Mutex::new(()),
            loaded_mtime: RwLock::new(None),
            defer_persist: AtomicBool::new(false),
            legacy_compose_hashes: Vec::new(),
        });
        let _ = store.load().await;
        store
    }

    fn lock_path(storage_path: &Path) -> PathBuf {
        let mut s = storage_path.as_os_str().to_owned();
        s.push(".lock");
        PathBuf::from(s)
    }

    /// Exclusive advisory lock for cross-process load/persist (sibling `.lock` file).
    ///
    /// Runs `flock` on the blocking pool so the async runtime is not stalled (important for
    /// `tokio::test` current-thread and for overlapping `schedule_persist` tasks).
    async fn acquire_disk_lock(&self) -> Result<std::fs::File, String> {
        let path = self
            .storage_path
            .as_ref()
            .ok_or_else(|| "persistence not configured".to_string())?
            .clone();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("create storage dir: {e}"))?;
        }
        let lock_path = Self::lock_path(&path);
        tokio::task::spawn_blocking(move || {
            let file = OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .truncate(false)
                .open(&lock_path)
                .map_err(|e| format!("open entitlements lock: {e}"))?;
            file.lock_exclusive()
                .map_err(|e| format!("lock entitlements file: {e}"))?;
            Ok(file)
        })
        .await
        .map_err(|e| format!("entitlements lock task: {e}"))?
    }

    fn disk_mtime(path: &Path) -> Option<SystemTime> {
        std::fs::metadata(path).ok().and_then(|m| m.modified().ok())
    }

    fn remember_mtime(&self, path: &Path) {
        *self.loaded_mtime.write().unwrap() = Self::disk_mtime(path);
    }

    /// Force reload from disk when persistence is configured. No-op for in-memory stores.
    pub async fn reload(&self) -> Result<(), String> {
        if self.storage_path.is_none() {
            return Ok(());
        }
        self.load().await.map(|_| ())
    }

    /// Reload when the on-disk file is newer than the last load/persist.
    ///
    /// Returns `true` when a reload ran. No-op for in-memory stores.
    pub async fn reload_if_stale(&self) -> Result<bool, String> {
        let Some(path) = self.storage_path.as_ref() else {
            return Ok(false);
        };
        let disk_mtime = Self::disk_mtime(path);
        let loaded = *self.loaded_mtime.read().unwrap();
        match (disk_mtime, loaded) {
            (None, _) => Ok(false),
            (Some(disk), Some(known)) if disk <= known => Ok(false),
            (Some(_), _) => {
                debug!("Entitlements file newer on disk; reloading");
                self.load().await?;
                Ok(true)
            }
        }
    }

    /// Hold the disk lock, reload from disk, run `f`, then persist.
    ///
    /// Prefer this for commerce (or any peer) read-modify-write so bot memory stays consistent.
    /// Mutations inside `f` that call [`Self::schedule_persist`] are deferred until this returns.
    pub async fn with_disk_lock<R>(
        self: &Arc<Self>,
        f: impl FnOnce(&Arc<Self>) -> R,
    ) -> Result<R, String> {
        if self.storage_path.is_none() {
            return Ok(f(self));
        }
        // Same lock order as load/persist: persist_lock then flock.
        let _guard = self.persist_lock.lock().await;
        let _disk = self.acquire_disk_lock().await?;
        self.load_unlocked().await?;
        self.defer_persist.store(true, Ordering::SeqCst);
        let out = f(self);
        self.defer_persist.store(false, Ordering::SeqCst);
        self.persist_unlocked().await?;
        Ok(out)
    }

    fn schedule_persist(self: &Arc<Self>) {
        if self.storage_path.is_none() {
            return;
        }
        if self.defer_persist.load(Ordering::SeqCst) {
            return;
        }
        let store = Arc::clone(self);
        tokio::spawn(async move {
            if let Err(e) = store.persist().await {
                warn!("Failed to persist entitlements: {e}");
            }
        });
    }

    /// Insert or replace a record under its owner (and group index when enabled).
    pub fn upsert(self: &Arc<Self>, record: EntitlementRecord) -> Result<(), String> {
        let owner = record
            .owner_uuid
            .clone()
            .ok_or_else(|| "upsert requires owner_uuid".to_string())?;
        let id = record.id.clone();

        {
            let mut individuals = self.individuals.write().unwrap();
            let list = individuals.entry(owner).or_default();
            if let Some(pos) = list.iter().position(|r| r.id == id) {
                list[pos] = record.clone();
            } else {
                list.push(record.clone());
            }
        }

        self.reindex_record_groups(&id, Some(&record));
        self.schedule_persist();
        Ok(())
    }

    /// Group ids this record sponsors (multi-enable + optional legacy claim).
    fn record_sponsored_groups(record: &EntitlementRecord) -> Vec<String> {
        let mut gids = record.enabled_group_ids.clone();
        if let Some(claimed) = record.claimed_group_id.as_ref() {
            if !claimed.is_empty() && !gids.iter().any(|g| g == claimed) {
                gids.push(claimed.clone());
            }
        }
        gids
    }

    /// Remove `record_id` from every group bucket, then re-add under its sponsored groups.
    fn reindex_record_groups(&self, record_id: &str, record: Option<&EntitlementRecord>) {
        let mut groups = self.groups.write().unwrap();
        for list in groups.values_mut() {
            list.retain(|r| r.id != record_id);
        }
        groups.retain(|_, list| !list.is_empty());

        let Some(rec) = record else {
            return;
        };
        if !rec.plan_sku.is_group_claimable() {
            return;
        }
        for gid in Self::record_sponsored_groups(rec) {
            groups.entry(gid).or_default().push(rec.clone());
        }
    }

    pub fn get_individual(&self, owner_key: &str) -> Vec<EntitlementRecord> {
        self.individuals
            .read()
            .unwrap()
            .get(owner_key)
            .cloned()
            .unwrap_or_default()
    }

    pub fn get_group(&self, group_id: &str) -> Vec<EntitlementRecord> {
        self.groups
            .read()
            .unwrap()
            .get(group_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn list_for_owner(&self, owner_key: &str) -> Vec<EntitlementRecord> {
        self.get_individual(owner_key)
    }

    pub fn get_pending(&self, link_token: &str) -> Option<EntitlementRecord> {
        self.pending_by_token
            .read()
            .unwrap()
            .get(link_token)
            .cloned()
    }

    /// Create a pending entitlement awaiting Signal link (`!link`).
    pub fn create_pending(
        self: &Arc<Self>,
        link_token: String,
        plan_sku: PlanSku,
        source: EntitlementSource,
        expires_at: Option<DateTime<Utc>>,
        stripe_customer_id: Option<String>,
        stripe_subscription_id: Option<String>,
    ) -> Result<EntitlementRecord, String> {
        if link_token.trim().is_empty() {
            return Err("link_token must be non-empty".into());
        }
        {
            let pending = self.pending_by_token.read().unwrap();
            if pending.contains_key(&link_token) {
                return Err(format!("link_token already pending: {link_token}"));
            }
        }

        let now = Utc::now();
        let record = EntitlementRecord {
            id: new_record_id(),
            plan_sku,
            source,
            status: EntitlementStatus::Active,
            expires_at,
            stripe_customer_id,
            stripe_subscription_id,
            owner_uuid: None,
            link_token: Some(link_token.clone()),
            claimed_group_id: None,
            enabled_group_ids: Vec::new(),
            created_at: now,
            updated_at: now,
        };

        self.pending_by_token
            .write()
            .unwrap()
            .insert(link_token, record.clone());
        self.schedule_persist();
        Ok(record)
    }

    /// Bind a pending `link_token` to a Signal owner; moves into `individuals`.
    pub fn bind_link_token(
        self: &Arc<Self>,
        link_token: &str,
        owner_uuid: String,
    ) -> Result<EntitlementRecord, String> {
        if owner_uuid.trim().is_empty() {
            return Err("owner_uuid must be non-empty".into());
        }
        let mut record = self
            .pending_by_token
            .write()
            .unwrap()
            .remove(link_token)
            .ok_or_else(|| format!("unknown link_token: {link_token}"))?;

        record.owner_uuid = Some(owner_uuid.clone());
        record.link_token = None;
        record.updated_at = Utc::now();

        {
            let mut individuals = self.individuals.write().unwrap();
            individuals
                .entry(owner_uuid)
                .or_default()
                .push(record.clone());
        }
        self.schedule_persist();
        Ok(record)
    }

    /// Enable Sigstack in a Signal group (alpha uncapped; paid packs respect `max_groups`).
    ///
    /// Idempotent when this owner's entitlement already sponsors `group_id`, or when
    /// any active sponsor already enabled the group.
    pub fn enable_sigstack(
        self: &Arc<Self>,
        owner_uuid: &str,
        group_id: String,
    ) -> Result<EntitlementRecord, String> {
        if group_id.trim().is_empty() {
            return Err("group_id must be non-empty".into());
        }
        let now = Utc::now();

        if self
            .get_group(&group_id)
            .iter()
            .any(|r| r.is_granting_at(now))
        {
            // Already enabled by someone; return caller's sponsoring row if any, else first.
            if let Some(mine) = self.get_individual(owner_uuid).into_iter().find(|r| {
                r.is_granting_at(now)
                    && r.plan_sku.is_group_claimable()
                    && Self::record_sponsored_groups(r)
                        .iter()
                        .any(|g| g == &group_id)
            }) {
                return Ok(mine);
            }
            return self
                .get_group(&group_id)
                .into_iter()
                .find(|r| r.is_granting_at(now))
                .ok_or_else(|| "group enabled but no granting sponsor found".into());
        }

        let mut individuals = self.individuals.write().unwrap();
        let list = individuals
            .get_mut(owner_uuid)
            .ok_or_else(|| format!("no entitlements for owner {owner_uuid}"))?;
        let pos = list
            .iter()
            .position(|r| r.plan_sku.is_group_claimable() && r.is_granting_at(now))
            .ok_or_else(|| {
                "no enableable entitlement found; link an alpha or all-access plan with !link <code> first"
                    .to_string()
            })?;

        if !list[pos].enabled_group_ids.iter().any(|g| g == &group_id) {
            if let Some(max) = list[pos].plan_sku.max_groups() {
                let current = Self::record_sponsored_groups(&list[pos]).len();
                if current >= max {
                    return Err(format!(
                        "this plan covers up to {max} groups; upgrade on the Plans page for more"
                    ));
                }
            }
            list[pos].enabled_group_ids.push(group_id.clone());
        }
        list[pos].updated_at = now;
        let record = list[pos].clone();
        drop(individuals);

        self.reindex_record_groups(&record.id, Some(&record));
        self.schedule_persist();
        Ok(record)
    }

    /// Paid one-group claim (sets `claimed_group_id`). Prefer [`Self::enable_sigstack`] for alpha.
    pub fn claim_group(
        self: &Arc<Self>,
        owner_uuid: &str,
        record_id: &str,
        group_id: String,
    ) -> Result<EntitlementRecord, String> {
        if group_id.trim().is_empty() {
            return Err("group_id must be non-empty".into());
        }

        let mut individuals = self.individuals.write().unwrap();
        let list = individuals
            .get_mut(owner_uuid)
            .ok_or_else(|| format!("no entitlements for owner {owner_uuid}"))?;
        let pos = list
            .iter()
            .position(|r| r.id == record_id)
            .ok_or_else(|| format!("record {record_id} not found for owner"))?;

        let sku = list[pos].plan_sku;
        if !sku.is_group_claimable() {
            return Err(format!("plan_sku {sku:?} is not claimable to a group"));
        }

        list[pos].claimed_group_id = Some(group_id.clone());
        if !list[pos].enabled_group_ids.iter().any(|g| g == &group_id) {
            list[pos].enabled_group_ids.push(group_id);
        }
        list[pos].updated_at = Utc::now();
        let record = list[pos].clone();
        drop(individuals);

        self.reindex_record_groups(&record.id, Some(&record));
        self.schedule_persist();
        Ok(record)
    }

    /// True when the owner has any active granting individual entitlement.
    pub fn has_active_individual(&self, owner_key: &str, now: DateTime<Utc>) -> bool {
        self.get_individual(owner_key)
            .iter()
            .any(|r| r.is_granting_at(now))
    }

    /// True when this group has at least one still-granting sponsor (was enabled).
    pub fn is_group_enabled(&self, group_id: &str, now: DateTime<Utc>) -> bool {
        self.get_group(group_id)
            .iter()
            .any(|r| r.is_granting_at(now))
    }

    pub fn set_status(
        self: &Arc<Self>,
        record_id: &str,
        status: EntitlementStatus,
    ) -> Result<EntitlementRecord, String> {
        let updated = self.mutate_record(record_id, |r| {
            r.status = status;
            r.updated_at = Utc::now();
        })?;
        self.schedule_persist();
        Ok(updated)
    }

    /// Mark granting records with `expires_at <= now` as `expired`. Returns count changed.
    pub fn expire_due(self: &Arc<Self>, now: DateTime<Utc>) -> usize {
        let mut changed = 0usize;
        {
            let mut individuals = self.individuals.write().unwrap();
            for list in individuals.values_mut() {
                for record in list.iter_mut() {
                    if matches!(
                        record.status,
                        EntitlementStatus::Active | EntitlementStatus::PastDue
                    ) {
                        if let Some(expires) = record.expires_at {
                            if expires <= now {
                                record.status = EntitlementStatus::Expired;
                                record.updated_at = now;
                                changed += 1;
                            }
                        }
                    }
                }
            }
        }
        {
            let mut pending = self.pending_by_token.write().unwrap();
            for record in pending.values_mut() {
                if matches!(
                    record.status,
                    EntitlementStatus::Active | EntitlementStatus::PastDue
                ) {
                    if let Some(expires) = record.expires_at {
                        if expires <= now {
                            record.status = EntitlementStatus::Expired;
                            record.updated_at = now;
                            changed += 1;
                        }
                    }
                }
            }
        }
        // Refresh group index from individuals (statuses may have changed).
        self.rebuild_group_index_from_individuals();
        if changed > 0 {
            self.schedule_persist();
        }
        changed
    }

    fn rebuild_group_index_from_individuals(&self) {
        let individuals = self.individuals.read().unwrap();
        let mut groups: HashMap<String, Vec<EntitlementRecord>> = HashMap::new();
        for list in individuals.values() {
            for record in list {
                if !record.plan_sku.is_group_claimable() {
                    continue;
                }
                for gid in Self::record_sponsored_groups(record) {
                    groups.entry(gid).or_default().push(record.clone());
                }
            }
        }
        *self.groups.write().unwrap() = groups;
    }

    fn mutate_record(
        &self,
        record_id: &str,
        f: impl FnOnce(&mut EntitlementRecord),
    ) -> Result<EntitlementRecord, String> {
        {
            let mut individuals = self.individuals.write().unwrap();
            for list in individuals.values_mut() {
                if let Some(record) = list.iter_mut().find(|r| r.id == record_id) {
                    f(record);
                    let updated = record.clone();
                    drop(individuals);
                    self.reindex_record_groups(record_id, Some(&updated));
                    return Ok(updated);
                }
            }
        }
        {
            let mut pending = self.pending_by_token.write().unwrap();
            if let Some(record) = pending.values_mut().find(|r| r.id == record_id) {
                f(record);
                return Ok(record.clone());
            }
        }
        Err(format!("record {record_id} not found"))
    }

    /// Effective grants for an owner in a group context (individual + that group's claims).
    pub fn effective_grants(
        &self,
        owner_key: &str,
        group_id: Option<&str>,
        now: DateTime<Utc>,
    ) -> HashMap<FeatureGrant, EntitlementSource> {
        let individual = self.get_individual(owner_key);
        let group = group_id.map(|g| self.get_group(g)).unwrap_or_default();
        compose_effective_grants(&individual, &group, now)
    }

    async fn derive_key(&self) -> Result<[u8; 32], String> {
        if let Some(key) = *self.cached_key.read().unwrap() {
            return Ok(key);
        }
        let (preferred, _) = self.encryption_keys().await?;
        *self.cached_key.write().unwrap() = Some(preferred);
        Ok(preferred)
    }

    async fn encryption_keys(&self) -> Result<([u8; 32], Vec<(String, [u8; 32])>), String> {
        let dstack = self
            .dstack
            .as_ref()
            .ok_or_else(|| "persistence not configured".to_string())?;

        let mut candidates = Vec::new();
        let mut derive_key = None;
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
                info!("Using DeriveKey endpoint for entitlements encryption");
                derive_key = Some(key);
                push_unique_key(&mut candidates, "DeriveKey".into(), key);
            }
            Err(e) => {
                warn!("DeriveKey not available for entitlements, using AppInfo fallback: {e}");
            }
        }

        let app_info = dstack
            .get_app_info()
            .await
            .map_err(|e| format!("Failed to get AppInfo for key derivation: {e}"))?;
        let app_id = app_info.app_id.as_deref().unwrap_or("unknown");
        let compose_hash = app_info.compose_hash.as_deref().unwrap_or("unknown");

        let stable = appinfo_fallback_key(app_id, None);
        info!("Using AppInfo-derived key for entitlements (app_id: {app_id}, no compose_hash)");
        push_unique_key(&mut candidates, "AppInfo app_id".into(), stable);
        push_unique_key(
            &mut candidates,
            format!("AppInfo compose_hash {compose_hash}"),
            appinfo_fallback_key(app_id, Some(compose_hash)),
        );
        for hash in &self.legacy_compose_hashes {
            push_unique_key(
                &mut candidates,
                format!("legacy compose_hash {hash}"),
                appinfo_fallback_key(app_id, Some(hash)),
            );
        }

        let preferred = derive_key.unwrap_or(stable);
        Ok((preferred, candidates))
    }

    fn snapshot(&self) -> EntitlementsSnapshot {
        EntitlementsSnapshot {
            version: DATA_VERSION,
            individuals: self.individuals.read().unwrap().clone(),
            groups: self.groups.read().unwrap().clone(),
            pending_by_token: self.pending_by_token.read().unwrap().clone(),
        }
    }

    async fn persist(&self) -> Result<(), String> {
        if self.storage_path.is_none() {
            return Err("persistence not configured".into());
        }
        // In-process first, then flock — avoids macOS same-process dual-fd flock deadlock.
        let _guard = self.persist_lock.lock().await;
        let _disk = self.acquire_disk_lock().await?;
        self.persist_unlocked().await
    }

    async fn persist_unlocked(&self) -> Result<(), String> {
        let path = self
            .storage_path
            .as_ref()
            .ok_or_else(|| "persistence not configured".to_string())?;

        let key = self.derive_key().await?;
        let data = encrypt_entitlements_blob(&self.snapshot(), &key)?;

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

        self.remember_mtime(path);
        debug!(
            "Saved encrypted entitlements ({} bytes) to {path:?}",
            data.len()
        );
        Ok(())
    }

    async fn load(&self) -> Result<usize, String> {
        if self.storage_path.is_none() {
            return Err("persistence not configured".into());
        }
        let _guard = self.persist_lock.lock().await;
        let _disk = self.acquire_disk_lock().await?;
        self.load_unlocked().await
    }

    async fn load_unlocked(&self) -> Result<usize, String> {
        let path = self
            .storage_path
            .as_ref()
            .ok_or_else(|| "persistence not configured".to_string())?;

        if !path.exists() {
            info!("Entitlements file not found at {path:?}, starting fresh");
            *self.loaded_mtime.write().unwrap() = None;
            return Ok(0);
        }

        let data = fs::read(path)
            .await
            .map_err(|e| format!("read entitlements: {e}"))?;

        let (snapshot, used_key, preferred_key) = self.decrypt_with_candidates(&data).await?;

        if snapshot.version != DATA_VERSION {
            warn!(
                "Entitlements version {} != expected {DATA_VERSION}",
                snapshot.version
            );
        }

        let count = snapshot.individuals.values().map(Vec::len).sum::<usize>()
            + snapshot.pending_by_token.len();
        *self.individuals.write().unwrap() = snapshot.individuals;
        *self.groups.write().unwrap() = snapshot.groups;
        *self.pending_by_token.write().unwrap() = snapshot.pending_by_token;
        *self.cached_key.write().unwrap() = Some(preferred_key);
        self.remember_mtime(path);
        if used_key != preferred_key {
            info!("Re-encrypting entitlements with the stable persist key");
            self.persist_unlocked().await?;
        }
        Ok(count)
    }

    async fn decrypt_with_candidates(
        &self,
        data: &[u8],
    ) -> Result<(EntitlementsSnapshot, [u8; 32], [u8; 32]), String> {
        if let Some(key) = *self.cached_key.read().unwrap() {
            let snapshot = decrypt_entitlements_blob(data, &key)?;
            return Ok((snapshot, key, key));
        }

        let (preferred, candidates) = self.encryption_keys().await?;
        let mut last_err = "no candidate keys".to_string();
        for (label, key) in candidates {
            match decrypt_entitlements_blob(data, &key) {
                Ok(snapshot) => {
                    info!("Decrypted entitlements with {label}");
                    return Ok((snapshot, key, preferred));
                }
                Err(e) => last_err = e,
            }
        }
        Err(last_err)
    }

    #[cfg(test)]
    pub async fn persist_now(&self) -> Result<(), String> {
        self.persist().await
    }

    /// Flush encrypted snapshot to disk (ops CLIs after minting).
    pub async fn flush(&self) -> Result<(), String> {
        self.persist().await
    }

    /// Mint `count` single-use pending alpha codes (`bundle-all-alpha`, +`days` expiry).
    /// Returns plaintext tokens (show once for `/alpha?code=` distribution).
    pub fn mint_alpha_codes(
        self: &Arc<Self>,
        count: usize,
        days: i64,
    ) -> Result<Vec<String>, String> {
        if count == 0 {
            return Err("count must be >= 1".into());
        }
        if days <= 0 {
            return Err("days must be >= 1".into());
        }
        let expires_at = Utc::now() + chrono::Duration::days(days);
        let mut codes = Vec::with_capacity(count);
        for _ in 0..count {
            let mut bytes = [0u8; 16];
            rand::thread_rng().fill_bytes(&mut bytes);
            let token = hex::encode(bytes);
            self.create_pending(
                token.clone(),
                PlanSku::BundleAllAlpha,
                EntitlementSource::Alpha,
                Some(expires_at),
                None,
                None,
            )?;
            codes.push(token);
        }
        Ok(codes)
    }

    /// Redeem the reusable friend code for `owner_uuid` (does not touch pending tokens).
    ///
    /// Grants a fresh `bundle-all-alpha` for 90 days. Fails if the owner already has an
    /// active alpha bundle grant.
    pub fn redeem_reusable_alpha(
        self: &Arc<Self>,
        owner_uuid: String,
    ) -> Result<EntitlementRecord, RedeemReuseError> {
        if owner_uuid.trim().is_empty() {
            return Err(RedeemReuseError::EmptyOwner);
        }
        let now = Utc::now();
        let owned = self.get_individual(&owner_uuid);
        if owned.iter().any(|r| {
            r.plan_sku == PlanSku::BundleAllAlpha
                && r.source == EntitlementSource::Alpha
                && r.is_granting_at(now)
        }) {
            return Err(RedeemReuseError::AlreadyActive);
        }

        let record = EntitlementRecord {
            id: new_record_id(),
            plan_sku: PlanSku::BundleAllAlpha,
            source: EntitlementSource::Alpha,
            status: EntitlementStatus::Active,
            expires_at: Some(now + chrono::Duration::days(90)),
            stripe_customer_id: None,
            stripe_subscription_id: None,
            owner_uuid: Some(owner_uuid),
            link_token: None,
            claimed_group_id: None,
            enabled_group_ids: Vec::new(),
            created_at: now,
            updated_at: now,
        };
        self.upsert(record.clone())
            .map_err(|_| RedeemReuseError::EmptyOwner)?;
        Ok(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use tempfile::tempdir;

    fn sample_record(owner: &str, sku: PlanSku, source: EntitlementSource) -> EntitlementRecord {
        let now = Utc::now();
        EntitlementRecord {
            id: new_record_id(),
            plan_sku: sku,
            source,
            status: EntitlementStatus::Active,
            expires_at: None,
            stripe_customer_id: None,
            stripe_subscription_id: None,
            owner_uuid: Some(owner.into()),
            link_token: None,
            claimed_group_id: None,
            enabled_group_ids: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn encrypted_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("entitlements.enc");
        let key = [7u8; 32];
        let dstack = DstackClient::new("/nonexistent/dstack.sock");

        let store = EntitlementsStore::with_test_key(dstack, path.clone(), key).await;
        let mut individual =
            sample_record("uuid-1", PlanSku::AllAccess3, EntitlementSource::Stripe);
        individual.stripe_customer_id = Some("cus_test".into());
        store.upsert(individual.clone()).unwrap();

        let mut group_sku =
            sample_record("uuid-1", PlanSku::AllAccess10, EntitlementSource::Stripe);
        group_sku.claimed_group_id = Some("group.internal".into());
        store.upsert(group_sku.clone()).unwrap();

        store.persist_now().await.unwrap();

        let store2 = EntitlementsStore::with_test_key(DstackClient::new("/x"), path, key).await;
        let loaded = store2.get_individual("uuid-1");
        assert_eq!(loaded.len(), 2);
        assert!(loaded.iter().any(|r| r.plan_sku == PlanSku::AllAccess3));
        assert!(loaded.iter().any(|r| {
            r.plan_sku == PlanSku::AllAccess10
                && r.claimed_group_id.as_deref() == Some("group.internal")
        }));
        assert_eq!(store2.get_group("group.internal").len(), 1);
    }

    #[test]
    fn expire_due_marks_past_expires_at() {
        let store = EntitlementsStore::new_in_memory();
        let now = Utc::now();
        let mut past = sample_record("u1", PlanSku::AllAccess3, EntitlementSource::Alpha);
        past.expires_at = Some(now - Duration::hours(1));
        let mut future = sample_record("u1", PlanSku::BundleAllAlpha, EntitlementSource::Alpha);
        future.expires_at = Some(now + Duration::hours(1));
        store.upsert(past.clone()).unwrap();
        store.upsert(future.clone()).unwrap();

        assert_eq!(store.expire_due(now), 1);
        let list = store.get_individual("u1");
        let past_rec = list.iter().find(|r| r.id == past.id).unwrap();
        let future_rec = list.iter().find(|r| r.id == future.id).unwrap();
        assert_eq!(past_rec.status, EntitlementStatus::Expired);
        assert_eq!(future_rec.status, EntitlementStatus::Active);
    }

    #[test]
    fn link_binding_moves_pending_to_individuals() {
        let store = EntitlementsStore::new_in_memory();
        let pending = store
            .create_pending(
                "tok-abc".into(),
                PlanSku::AllAccess3,
                EntitlementSource::Stripe,
                None,
                Some("cus_1".into()),
                Some("sub_1".into()),
            )
            .unwrap();
        assert!(store.get_pending("tok-abc").is_some());
        assert!(store.get_individual("uuid-owner").is_empty());

        let bound = store
            .bind_link_token("tok-abc", "uuid-owner".into())
            .unwrap();
        assert_eq!(bound.id, pending.id);
        assert_eq!(bound.owner_uuid.as_deref(), Some("uuid-owner"));
        assert!(bound.link_token.is_none());
        assert!(store.get_pending("tok-abc").is_none());
        assert_eq!(store.get_individual("uuid-owner").len(), 1);
    }

    #[test]
    fn claim_group_indexes_under_groups() {
        let store = EntitlementsStore::new_in_memory();
        let record = sample_record("owner-1", PlanSku::AllAccess3, EntitlementSource::Stripe);
        let id = record.id.clone();
        store.upsert(record).unwrap();

        let claimed = store
            .claim_group("owner-1", &id, "group.main".into())
            .unwrap();
        assert_eq!(claimed.claimed_group_id.as_deref(), Some("group.main"));
        assert!(claimed.enabled_group_ids.iter().any(|g| g == "group.main"));
        assert_eq!(store.get_group("group.main").len(), 1);
        assert_eq!(
            store.get_individual("owner-1")[0]
                .claimed_group_id
                .as_deref(),
            Some("group.main")
        );
    }

    #[test]
    fn enable_sigstack_allows_multiple_groups_for_alpha() {
        let store = EntitlementsStore::new_in_memory();
        store.redeem_reusable_alpha("owner-1".into()).unwrap();

        let a = store.enable_sigstack("owner-1", "group.a".into()).unwrap();
        assert!(a.enabled_group_ids.iter().any(|g| g == "group.a"));
        let b = store.enable_sigstack("owner-1", "group.b".into()).unwrap();
        assert!(b.enabled_group_ids.iter().any(|g| g == "group.a"));
        assert!(b.enabled_group_ids.iter().any(|g| g == "group.b"));
        assert_eq!(store.get_group("group.a").len(), 1);
        assert_eq!(store.get_group("group.b").len(), 1);
        assert!(store.is_group_enabled("group.a", Utc::now()));
        assert!(store.is_group_enabled("group.b", Utc::now()));
    }

    #[test]
    fn enable_sigstack_idempotent_when_already_enabled() {
        let store = EntitlementsStore::new_in_memory();
        store.redeem_reusable_alpha("owner-1".into()).unwrap();
        store
            .enable_sigstack("owner-1", "group.main".into())
            .unwrap();
        let again = store
            .enable_sigstack("owner-1", "group.main".into())
            .unwrap();
        assert_eq!(
            again
                .enabled_group_ids
                .iter()
                .filter(|g| *g == "group.main")
                .count(),
            1
        );
        // Another entitled user also gets idempotent success.
        store.redeem_reusable_alpha("owner-2".into()).unwrap();
        let other = store
            .enable_sigstack("owner-2", "group.main".into())
            .unwrap();
        assert!(other.is_granting_at(Utc::now()));
    }

    #[test]
    fn enabled_group_ids_default_on_legacy_json() {
        let json = r#"{
            "id":"r1",
            "plan_sku":"bundle-all-alpha",
            "source":"alpha",
            "status":"active",
            "owner_uuid":"u1",
            "created_at":"2026-01-01T00:00:00Z",
            "updated_at":"2026-01-01T00:00:00Z"
        }"#;
        let rec: EntitlementRecord = serde_json::from_str(json).unwrap();
        assert!(rec.enabled_group_ids.is_empty());
    }

    #[test]
    fn composition_paid_replaces_alpha_on_overlap() {
        let now = Utc::now();
        let alpha = {
            let mut r = sample_record("u", PlanSku::BundleAllAlpha, EntitlementSource::Alpha);
            r.expires_at = Some(now + Duration::days(90));
            r
        };
        let paid = sample_record("u", PlanSku::AllAccess3, EntitlementSource::Stripe);

        let grants = compose_effective_grants(&[alpha, paid], &[], now);
        // Paid all-access overlaps every feature → Stripe wins across the board.
        for grant in ALL_ACCESS_GRANTS {
            assert_eq!(
                grants.get(grant),
                Some(&EntitlementSource::Stripe),
                "{grant:?}"
            );
        }
    }

    #[test]
    fn decrypts_blob_encrypted_with_legacy_compose_hash_key() {
        let mut individuals = HashMap::new();
        individuals.insert(
            "u1".into(),
            vec![sample_record(
                "u1",
                PlanSku::AllAccess3,
                EntitlementSource::Stripe,
            )],
        );
        let snapshot = EntitlementsSnapshot {
            version: DATA_VERSION,
            individuals,
            groups: HashMap::new(),
            pending_by_token: HashMap::new(),
        };
        let legacy = appinfo_fallback_key("app-1", Some("old-compose"));
        let stable = appinfo_fallback_key("app-1", None);
        let blob = encrypt_entitlements_blob(&snapshot, &legacy).unwrap();

        assert!(decrypt_entitlements_blob(&blob, &stable).is_err());
        let loaded = decrypt_entitlements_blob(&blob, &legacy).unwrap();
        assert_eq!(loaded.individuals["u1"].len(), 1);
    }

    #[test]
    fn mint_alpha_codes_creates_pending() {
        let store = EntitlementsStore::new_in_memory();
        let codes = store.mint_alpha_codes(3, 90).unwrap();
        assert_eq!(codes.len(), 3);
        for code in &codes {
            let pending = store.get_pending(code).unwrap();
            assert_eq!(pending.plan_sku, PlanSku::BundleAllAlpha);
            assert_eq!(pending.source, EntitlementSource::Alpha);
            assert!(pending.expires_at.is_some());
        }
    }

    #[test]
    fn reusable_alpha_code_match_is_case_insensitive() {
        assert!(is_reusable_alpha_code("bread"));
        assert!(is_reusable_alpha_code("Bread"));
        assert!(is_reusable_alpha_code("  BREAD  "));
        assert!(!is_reusable_alpha_code("bread-fiend"));
    }

    #[test]
    fn redeem_reusable_alpha_allows_two_owners() {
        let store = EntitlementsStore::new_in_memory();
        let a = store.redeem_reusable_alpha("uuid-a".into()).unwrap();
        let b = store.redeem_reusable_alpha("uuid-b".into()).unwrap();
        assert_eq!(a.plan_sku, PlanSku::BundleAllAlpha);
        assert_eq!(b.source, EntitlementSource::Alpha);
        assert_eq!(store.get_individual("uuid-a").len(), 1);
        assert_eq!(store.get_individual("uuid-b").len(), 1);
        assert!(store.pending_by_token.read().unwrap().is_empty());
    }

    #[test]
    fn redeem_reusable_alpha_rejects_while_active() {
        let store = EntitlementsStore::new_in_memory();
        store.redeem_reusable_alpha("uuid-a".into()).unwrap();
        assert_eq!(
            store.redeem_reusable_alpha("uuid-a".into()).unwrap_err(),
            RedeemReuseError::AlreadyActive
        );
    }

    #[test]
    fn redeem_reusable_alpha_allows_again_after_expiry() {
        let store = EntitlementsStore::new_in_memory();
        let first = store.redeem_reusable_alpha("uuid-a".into()).unwrap();
        // Force expiry on the active row.
        store
            .set_status(&first.id, EntitlementStatus::Expired)
            .unwrap();
        let again = store.redeem_reusable_alpha("uuid-a".into()).unwrap();
        assert_ne!(first.id, again.id);
        assert_eq!(again.status, EntitlementStatus::Active);
        assert_eq!(store.get_individual("uuid-a").len(), 2);
    }

    #[test]
    fn plan_sku_serde_matches_site_ids() {
        assert_eq!(
            serde_json::to_string(&PlanSku::AllAccess3).unwrap(),
            "\"all-access-3\""
        );
        assert_eq!(
            serde_json::to_string(&PlanSku::AllAccess10).unwrap(),
            "\"all-access-10\""
        );
        assert_eq!(
            serde_json::to_string(&PlanSku::BundleAllAlpha).unwrap(),
            "\"bundle-all-alpha\""
        );
        assert_eq!(PlanSku::AllAccess3.max_groups(), Some(3));
        assert_eq!(PlanSku::AllAccess10.max_groups(), Some(10));
        assert_eq!(PlanSku::BundleAllAlpha.max_groups(), None);
    }

    #[test]
    fn appinfo_key_without_compose_hash_differs_from_legacy_mix() {
        let stable = appinfo_fallback_key("app-1", None);
        let legacy = appinfo_fallback_key("app-1", Some("old-compose"));
        assert_ne!(stable, legacy);
    }

    #[test]
    fn enable_sigstack_enforces_all_access_3_cap() {
        let store = EntitlementsStore::new_in_memory();
        let record = sample_record("owner-1", PlanSku::AllAccess3, EntitlementSource::Stripe);
        store.upsert(record).unwrap();

        for gid in ["group.a", "group.b", "group.c"] {
            store
                .enable_sigstack("owner-1", gid.into())
                .unwrap_or_else(|e| panic!("enable {gid}: {e}"));
        }
        let err = store
            .enable_sigstack("owner-1", "group.d".into())
            .unwrap_err();
        assert!(err.contains("up to 3 groups"), "{err}");
        assert!(!store.is_group_enabled("group.d", Utc::now()));
        // Idempotent re-enable of an existing group still succeeds.
        store.enable_sigstack("owner-1", "group.a".into()).unwrap();
    }

    #[test]
    fn enable_sigstack_enforces_all_access_10_cap() {
        let store = EntitlementsStore::new_in_memory();
        let record = sample_record("owner-1", PlanSku::AllAccess10, EntitlementSource::Stripe);
        store.upsert(record).unwrap();

        for i in 0..10 {
            let gid = format!("group.{i}");
            store.enable_sigstack("owner-1", gid).unwrap();
        }
        let err = store
            .enable_sigstack("owner-1", "group.overflow".into())
            .unwrap_err();
        assert!(err.contains("up to 10 groups"), "{err}");
    }

    #[tokio::test]
    async fn reload_if_stale_picks_up_peer_write() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("entitlements.enc");
        let key = [9u8; 32];

        let writer =
            EntitlementsStore::with_test_key(DstackClient::new("/x"), path.clone(), key).await;
        let reader =
            EntitlementsStore::with_test_key(DstackClient::new("/x"), path.clone(), key).await;

        assert!(reader.get_pending("tok-peer").is_none());

        writer
            .create_pending(
                "tok-peer".into(),
                PlanSku::AllAccess3,
                EntitlementSource::Stripe,
                None,
                None,
                None,
            )
            .unwrap();
        writer.flush().await.unwrap();

        // Ensure mtime advances on coarse filesystems.
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        // Touch via another persist if needed — flush already wrote; bump by rewriting.
        writer.flush().await.unwrap();

        assert!(reader.reload_if_stale().await.unwrap());
        assert!(reader.get_pending("tok-peer").is_some());
        assert!(!reader.reload_if_stale().await.unwrap());
    }

    #[tokio::test]
    async fn with_disk_lock_persists_create_pending() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("entitlements.enc");
        let key = [11u8; 32];

        let store =
            EntitlementsStore::with_test_key(DstackClient::new("/x"), path.clone(), key).await;
        store
            .with_disk_lock(|s| {
                s.create_pending(
                    "tok-lock".into(),
                    PlanSku::AllAccess10,
                    EntitlementSource::Stripe,
                    None,
                    Some("cus".into()),
                    None,
                )
                .unwrap();
            })
            .await
            .unwrap();

        let reopened = EntitlementsStore::with_test_key(DstackClient::new("/x"), path, key).await;
        assert!(reopened.get_pending("tok-lock").is_some());
    }

    #[tokio::test]
    async fn reload_if_stale_noop_for_in_memory() {
        let store = EntitlementsStore::new_in_memory();
        assert!(!store.reload_if_stale().await.unwrap());
        store.reload().await.unwrap();
    }
}
