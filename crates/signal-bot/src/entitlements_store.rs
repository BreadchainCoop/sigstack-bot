//! Encrypted entitlement records (paid / alpha plans), separate from feature prefs.
//!
//! # Plan composition (alpha vs paid)
//!
//! Alpha `bundle-all-alpha` grants full bundle feature coverage while `active`/`past_due`
//! and unexpired. A **paid** (`source: stripe`) entitlement that overlaps a feature
//! **replaces** alpha for that feature only; non-overlapping alpha coverage remains.
//! Stripe catalog SKUs must match the site offer ids in `site/src/lib/content/en.ts`
//! (plus `bundle-all-alpha`); do not invent `-me` aliases here.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use chrono::{DateTime, Utc};
use dstack_client::DstackClient;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::fs;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

const DATA_VERSION: u32 = 1;
const KEY_DERIVATION_PATH: &str = "signal-bot/entitlements";
const NONCE_SIZE: usize = 12;

/// Plan SKU strings aligned with `site/src/lib/content/en.ts` offer `id`s, plus alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlanSku {
    BundleIndividual,
    BundleGroup,
    ThreadsIndividual,
    ThreadsGroup,
    InChatMe,
    InChatAll,
    TranscriptionIndividual,
    /// Commerce alpha: full Bundle-all for a limited window (`source: alpha`).
    BundleAllAlpha,
}

impl PlanSku {
    /// Group-scoped catalog SKUs (claimed against a Signal group `internal_id`).
    pub fn is_group_scope(self) -> bool {
        matches!(
            self,
            Self::BundleGroup | Self::ThreadsGroup | Self::InChatAll
        )
    }

    /// Feature grants implied by this SKU (before alpha/paid composition).
    pub fn grants(self) -> &'static [FeatureGrant] {
        match self {
            Self::BundleIndividual => &[
                FeatureGrant::ThreadsIndividual,
                FeatureGrant::InChatMe,
                FeatureGrant::TranscriptionIndividual,
            ],
            Self::BundleAllAlpha => &[
                FeatureGrant::ThreadsIndividual,
                FeatureGrant::InChatMe,
                FeatureGrant::TranscriptionIndividual,
                FeatureGrant::ThreadsGroup,
                FeatureGrant::InChatAll,
            ],
            Self::BundleGroup => &[FeatureGrant::ThreadsGroup, FeatureGrant::InChatAll],
            Self::ThreadsIndividual => &[FeatureGrant::ThreadsIndividual],
            Self::ThreadsGroup => &[FeatureGrant::ThreadsGroup],
            Self::InChatMe => &[FeatureGrant::InChatMe],
            Self::InChatAll => &[FeatureGrant::InChatAll],
            Self::TranscriptionIndividual => &[FeatureGrant::TranscriptionIndividual],
        }
    }

    /// Whether this SKU may be claimed onto a Signal group.
    pub fn is_group_claimable(self) -> bool {
        self.is_group_scope() || matches!(self, Self::BundleAllAlpha)
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claimed_group_id: Option<String>,
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
pub const REUSABLE_ALPHA_CODE: &str = "bread-friend";

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
            // bundle-all-alpha includes group features even on individual records.
            if record.plan_sku.is_group_scope() && record.claimed_group_id.is_none() {
                // Unclaimed group SKU does not yet grant group features.
                if matches!(grant, FeatureGrant::ThreadsGroup | FeatureGrant::InChatAll) {
                    continue;
                }
            }
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
            legacy_compose_hashes: Vec::new(),
        });
        let _ = store.load().await;
        store
    }

    fn schedule_persist(self: &Arc<Self>) {
        if self.storage_path.is_none() {
            return;
        }
        let store = Arc::clone(self);
        tokio::spawn(async move {
            if let Err(e) = store.persist().await {
                warn!("Failed to persist entitlements: {e}");
            }
        });
    }

    /// Insert or replace a record under its owner (and group index when claimed).
    pub fn upsert(self: &Arc<Self>, record: EntitlementRecord) -> Result<(), String> {
        let owner = record
            .owner_uuid
            .clone()
            .ok_or_else(|| "upsert requires owner_uuid".to_string())?;
        let id = record.id.clone();
        let claimed = record.claimed_group_id.clone();

        {
            let mut individuals = self.individuals.write().unwrap();
            let list = individuals.entry(owner).or_default();
            if let Some(pos) = list.iter().position(|r| r.id == id) {
                list[pos] = record.clone();
            } else {
                list.push(record.clone());
            }
        }

        self.reindex_group_claim(&id, claimed.as_deref(), Some(&record));
        self.schedule_persist();
        Ok(())
    }

    fn reindex_group_claim(
        &self,
        record_id: &str,
        new_group: Option<&str>,
        record: Option<&EntitlementRecord>,
    ) {
        let mut groups = self.groups.write().unwrap();
        for list in groups.values_mut() {
            list.retain(|r| r.id != record_id);
        }
        groups.retain(|_, list| !list.is_empty());

        if let (Some(gid), Some(rec)) = (new_group, record) {
            if rec.plan_sku.is_group_claimable() {
                groups.entry(gid.to_string()).or_default().push(rec.clone());
            }
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

    /// Claim a group-scoped (or alpha) entitlement onto a Signal group.
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
        list[pos].updated_at = Utc::now();
        let record = list[pos].clone();
        drop(individuals);

        self.reindex_group_claim(record_id, Some(&group_id), Some(&record));
        self.schedule_persist();
        Ok(record)
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
                if let Some(gid) = &record.claimed_group_id {
                    groups.entry(gid.clone()).or_default().push(record.clone());
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
                    let claimed = updated.claimed_group_id.clone();
                    drop(individuals);
                    self.reindex_group_claim(record_id, claimed.as_deref(), Some(&updated));
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
        let _guard = self.persist_lock.lock().await;

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

        debug!(
            "Saved encrypted entitlements ({} bytes) to {path:?}",
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
            info!("Entitlements file not found at {path:?}, starting fresh");
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
        if used_key != preferred_key {
            info!("Re-encrypting entitlements with the stable persist key");
            self.persist().await?;
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
        let mut individual = sample_record("uuid-1", PlanSku::InChatMe, EntitlementSource::Stripe);
        individual.stripe_customer_id = Some("cus_test".into());
        store.upsert(individual.clone()).unwrap();

        let mut group_sku =
            sample_record("uuid-1", PlanSku::ThreadsGroup, EntitlementSource::Stripe);
        group_sku.claimed_group_id = Some("group.internal".into());
        store.upsert(group_sku.clone()).unwrap();

        store.persist_now().await.unwrap();

        let store2 = EntitlementsStore::with_test_key(DstackClient::new("/x"), path, key).await;
        let loaded = store2.get_individual("uuid-1");
        assert_eq!(loaded.len(), 2);
        assert!(loaded.iter().any(|r| r.plan_sku == PlanSku::InChatMe));
        assert!(loaded.iter().any(|r| r.plan_sku == PlanSku::ThreadsGroup
            && r.claimed_group_id.as_deref() == Some("group.internal")));
        assert_eq!(store2.get_group("group.internal").len(), 1);
    }

    #[test]
    fn expire_due_marks_past_expires_at() {
        let store = EntitlementsStore::new_in_memory();
        let now = Utc::now();
        let mut past = sample_record("u1", PlanSku::InChatMe, EntitlementSource::Alpha);
        past.expires_at = Some(now - Duration::hours(1));
        let mut future = sample_record("u1", PlanSku::ThreadsIndividual, EntitlementSource::Alpha);
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
                PlanSku::BundleIndividual,
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
        let record = sample_record("owner-1", PlanSku::InChatAll, EntitlementSource::Stripe);
        let id = record.id.clone();
        store.upsert(record).unwrap();

        let claimed = store
            .claim_group("owner-1", &id, "group.main".into())
            .unwrap();
        assert_eq!(claimed.claimed_group_id.as_deref(), Some("group.main"));
        assert_eq!(store.get_group("group.main").len(), 1);
        assert_eq!(
            store.get_individual("owner-1")[0]
                .claimed_group_id
                .as_deref(),
            Some("group.main")
        );
    }

    #[test]
    fn composition_paid_replaces_alpha_on_overlap() {
        let now = Utc::now();
        let alpha = {
            let mut r = sample_record("u", PlanSku::BundleAllAlpha, EntitlementSource::Alpha);
            r.expires_at = Some(now + Duration::days(90));
            r
        };
        let paid = sample_record("u", PlanSku::InChatMe, EntitlementSource::Stripe);

        let grants = compose_effective_grants(&[alpha, paid], &[], now);
        assert_eq!(
            grants.get(&FeatureGrant::InChatMe),
            Some(&EntitlementSource::Stripe)
        );
        assert_eq!(
            grants.get(&FeatureGrant::ThreadsIndividual),
            Some(&EntitlementSource::Alpha)
        );
        assert_eq!(
            grants.get(&FeatureGrant::TranscriptionIndividual),
            Some(&EntitlementSource::Alpha)
        );
    }

    #[test]
    fn decrypts_blob_encrypted_with_legacy_compose_hash_key() {
        let mut individuals = HashMap::new();
        individuals.insert(
            "u1".into(),
            vec![sample_record(
                "u1",
                PlanSku::TranscriptionIndividual,
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
        assert!(is_reusable_alpha_code("bread-friend"));
        assert!(is_reusable_alpha_code("Bread-Friend"));
        assert!(is_reusable_alpha_code("  BREAD-FRIEND  "));
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
            serde_json::to_string(&PlanSku::BundleIndividual).unwrap(),
            "\"bundle-individual\""
        );
        assert_eq!(
            serde_json::to_string(&PlanSku::InChatMe).unwrap(),
            "\"in-chat-me\""
        );
        assert_eq!(
            serde_json::to_string(&PlanSku::BundleAllAlpha).unwrap(),
            "\"bundle-all-alpha\""
        );
    }

    #[test]
    fn appinfo_key_without_compose_hash_differs_from_legacy_mix() {
        let stable = appinfo_fallback_key("app-1", None);
        let legacy = appinfo_fallback_key("app-1", Some("old-compose"));
        assert_ne!(stable, legacy);
    }

    #[test]
    fn unclaimed_group_sku_does_not_grant_group_features() {
        let now = Utc::now();
        let unclaimed = sample_record("u", PlanSku::InChatAll, EntitlementSource::Stripe);
        let grants = compose_effective_grants(&[unclaimed], &[], now);
        assert!(!grants.contains_key(&FeatureGrant::InChatAll));
    }
}
