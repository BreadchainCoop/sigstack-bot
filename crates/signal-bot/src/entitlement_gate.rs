//! Access checks for entitlement enforcement (`ENTITLEMENTS__ENFORCE`).
//!
//! # Coarse MVP (this module)
//!
//! When `enforce` is true, every matched handler (except `!link`) must pass a
//! binary check before `execute` runs in [`crate::dispatch::dispatch_message`]:
//!
//! - **DM:** caller needs an active individual entitlement (`has_active_individual`).
//! - **Group:** the Signal group must be enabled (`is_group_enabled`) via
//!   `!enable-sigstack` by a linked owner.
//! - **`!enable-sigstack`:** requires an active individual entitlement first.
//!
//! Local/dev: `ENTITLEMENTS__ENFORCE=false` (compose default). Phala may set
//! `true`; flip prod only after commerce E2E.
//!
//! # Deferred: per-feature grants
//!
//! [`crate::entitlements_store::FeatureGrant`] / `effective_grants` / composition
//! (“paid wins on overlap”) exist in the store but are **not** consulted here.
//! All-access packs grant every product together once the group or DM is unlocked.
//! Wire handler → `FeatureGrant` mapping in a later change if SKUs diverge.

use crate::entitlements_store::EntitlementsStore;
use chrono::Utc;
use signal_bot_core::starts_with_word;
use signal_client::BotMessage;
use std::sync::Arc;

pub const DENY_NEED_LINK: &str =
    "Access locked. DM this bot with !link <code> to unlock (checkout code or alpha).";

pub const DENY_NEED_ENABLE: &str =
    "Sigstack is not enabled in this group yet. A linked member must run !enable-sigstack.";

/// Why access was denied (for reply copy).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateDeny {
    NeedLink,
    NeedEnable,
}

impl GateDeny {
    pub fn message(self) -> &'static str {
        match self {
            Self::NeedLink => DENY_NEED_LINK,
            Self::NeedEnable => DENY_NEED_ENABLE,
        }
    }
}

fn owner_key(message: &BotMessage) -> String {
    message.source.trim().to_string()
}

/// Decide whether a matched handler may run when enforcement is on.
///
/// Returns `Ok(())` when allowed, or `Err(GateDeny)` with user-facing copy.
pub fn check_access(store: &EntitlementsStore, message: &BotMessage) -> Result<(), GateDeny> {
    let text = message.text.as_str();
    if starts_with_word(text, "!link") {
        return Ok(());
    }

    let now = Utc::now();
    let owner = owner_key(message);
    let entitled = !owner.is_empty() && store.has_active_individual(&owner, now);

    if starts_with_word(text, "!enable-sigstack") {
        return if entitled {
            Ok(())
        } else {
            Err(GateDeny::NeedLink)
        };
    }

    if message.is_group {
        if let Some(gid) = message.group_id.as_deref() {
            if store.is_group_enabled(gid, now) {
                return Ok(());
            }
        }
        // Entitled users in a non-enabled group may still only link/enable (handled above).
        return Err(GateDeny::NeedEnable);
    }

    // DM: personal entitlement required.
    if entitled {
        Ok(())
    } else {
        Err(GateDeny::NeedLink)
    }
}

/// Gate context passed into dispatch.
#[derive(Clone)]
pub struct EntitlementGate {
    pub store: Arc<EntitlementsStore>,
    pub enforce: bool,
}

impl EntitlementGate {
    pub async fn allow(&self, message: &BotMessage) -> Result<(), GateDeny> {
        if !self.enforce {
            return Ok(());
        }
        if let Err(e) = self.store.reload_if_stale().await {
            tracing::warn!("entitlements reload before gate failed: {e}");
        }
        check_access(&self.store, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entitlements_store::{
        EntitlementRecord, EntitlementSource, EntitlementStatus, PlanSku, REUSABLE_ALPHA_CODE,
    };
    use chrono::{Duration, Utc};

    fn dm(text: &str, source: &str) -> BotMessage {
        BotMessage {
            source: source.into(),
            source_number: Some("+15551234567".into()),
            source_name: None,
            text: text.into(),
            timestamp: 1,
            message_timestamp: 1,
            is_group: false,
            group_id: None,
            group_name: None,
            receiving_account: "+15550000000".into(),
            attachments: vec![],
            quote: None,
        }
    }

    fn group(text: &str, source: &str, group_id: &str) -> BotMessage {
        let mut m = dm(text, source);
        m.is_group = true;
        m.group_id = Some(group_id.into());
        m
    }

    fn paid_active(owner: &str, sku: PlanSku) -> EntitlementRecord {
        let now = Utc::now();
        EntitlementRecord {
            id: format!("rec-{owner}"),
            plan_sku: sku,
            source: EntitlementSource::Stripe,
            status: EntitlementStatus::Active,
            expires_at: None,
            stripe_customer_id: Some("cus_x".into()),
            stripe_subscription_id: Some("sub_x".into()),
            owner_uuid: Some(owner.into()),
            link_token: None,
            claimed_group_id: None,
            enabled_group_ids: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Representative product commands from the gating map (coarse: all share one check).
    const PRODUCT_DM_CMDS: &[&str] = &[
        "!translate-me-on es",
        "!translate-me-thread es",
        "!transcribe-on",
        "!help",
    ];
    const PRODUCT_GROUP_CMDS: &[&str] = &[
        "!translate-all-on es",
        "!translate-me-on es",
        "!transcribe-on",
        "!help",
    ];

    #[test]
    fn link_always_allowed() {
        let store = EntitlementsStore::new_in_memory();
        assert!(check_access(&store, &dm("!link abc", "u")).is_ok());
        assert!(check_access(&store, &group("!link abc", "u", "g1")).is_ok());
    }

    #[test]
    fn unlinked_dm_product_commands_denied() {
        let store = EntitlementsStore::new_in_memory();
        for cmd in PRODUCT_DM_CMDS {
            assert_eq!(
                check_access(&store, &dm(cmd, "u")).unwrap_err(),
                GateDeny::NeedLink,
                "{cmd}"
            );
        }
    }

    #[test]
    fn linked_dm_product_commands_allowed() {
        let store = EntitlementsStore::new_in_memory();
        store
            .upsert(paid_active("uuid-ada", PlanSku::AllAccess3))
            .unwrap();
        for cmd in PRODUCT_DM_CMDS {
            assert!(check_access(&store, &dm(cmd, "uuid-ada")).is_ok(), "{cmd}");
        }
    }

    #[test]
    fn expired_dm_product_commands_denied() {
        let store = EntitlementsStore::new_in_memory();
        let mut expired = paid_active("uuid-ada", PlanSku::AllAccess3);
        expired.status = EntitlementStatus::Expired;
        expired.expires_at = Some(Utc::now() - Duration::days(1));
        store.upsert(expired).unwrap();
        for cmd in PRODUCT_DM_CMDS {
            assert_eq!(
                check_access(&store, &dm(cmd, "uuid-ada")).unwrap_err(),
                GateDeny::NeedLink,
                "{cmd}"
            );
        }
    }

    #[test]
    fn group_not_enabled_denies_product_commands() {
        let store = EntitlementsStore::new_in_memory();
        store
            .upsert(paid_active("uuid-ada", PlanSku::AllAccess3))
            .unwrap();
        for cmd in PRODUCT_GROUP_CMDS {
            assert_eq!(
                check_access(&store, &group(cmd, "uuid-ada", "g1")).unwrap_err(),
                GateDeny::NeedEnable,
                "{cmd}"
            );
        }
    }

    #[test]
    fn group_enabled_allows_any_member_product_commands() {
        let store = EntitlementsStore::new_in_memory();
        store
            .upsert(paid_active("uuid-ada", PlanSku::AllAccess10))
            .unwrap();
        store.enable_sigstack("uuid-ada", "g1".into()).unwrap();
        for cmd in PRODUCT_GROUP_CMDS {
            assert!(
                check_access(&store, &group(cmd, "uuid-bob", "g1")).is_ok(),
                "{cmd}"
            );
        }
    }

    #[test]
    fn alpha_dm_allowed_like_paid() {
        let store = EntitlementsStore::new_in_memory();
        store.redeem_reusable_alpha("uuid-ada".into()).unwrap();
        assert!(check_access(&store, &dm("!translate-me-on es", "uuid-ada")).is_ok());
    }

    #[test]
    fn presence_without_enable_denies_group_help() {
        let store = EntitlementsStore::new_in_memory();
        store.redeem_reusable_alpha("uuid-ada".into()).unwrap();
        assert_eq!(
            check_access(&store, &group("!help", "uuid-ada", "g1")).unwrap_err(),
            GateDeny::NeedEnable
        );
        assert_eq!(
            check_access(&store, &group("!help", "uuid-bob", "g1")).unwrap_err(),
            GateDeny::NeedEnable
        );
    }

    #[test]
    fn after_enable_any_member_allowed() {
        let store = EntitlementsStore::new_in_memory();
        store.redeem_reusable_alpha("uuid-ada".into()).unwrap();
        store.enable_sigstack("uuid-ada", "g1".into()).unwrap();
        assert!(check_access(&store, &group("!help", "uuid-bob", "g1")).is_ok());
        assert!(check_access(&store, &group("!help", "uuid-ada", "g1")).is_ok());
    }

    #[test]
    fn enable_requires_entitlement() {
        let store = EntitlementsStore::new_in_memory();
        assert_eq!(
            check_access(&store, &group("!enable-sigstack", "bob", "g1")).unwrap_err(),
            GateDeny::NeedLink
        );
        store.redeem_reusable_alpha("ada".into()).unwrap();
        assert!(check_access(&store, &group("!enable-sigstack", "ada", "g1")).is_ok());
    }

    #[tokio::test]
    async fn gate_enforce_false_always_allows() {
        let store = EntitlementsStore::new_in_memory();
        let gate = EntitlementGate {
            store,
            enforce: false,
        };
        assert!(gate.allow(&dm("!help", "nobody")).await.is_ok());
        assert!(gate
            .allow(&group("!translate-all-on es", "nobody", "g1"))
            .await
            .is_ok());
    }

    #[test]
    fn reusable_code_constant_still_usable() {
        assert_eq!(REUSABLE_ALPHA_CODE, "bread");
    }

    #[test]
    fn deny_copy_mentions_checkout_or_alpha() {
        assert!(DENY_NEED_LINK.contains("checkout") || DENY_NEED_LINK.contains("alpha"));
        assert!(DENY_NEED_ENABLE.contains("!enable-sigstack"));
    }
}
