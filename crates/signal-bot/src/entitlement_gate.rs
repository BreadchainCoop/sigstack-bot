//! Access checks for entitlement enforcement (`ENTITLEMENTS__ENFORCE`).

use crate::entitlements_store::EntitlementsStore;
use chrono::Utc;
use signal_bot_core::starts_with_word;
use signal_client::BotMessage;
use std::sync::Arc;

pub const DENY_NEED_LINK: &str =
    "Access locked. DM this bot with !link <code> to unlock. Ask for an alpha code if you do not have one.";

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
    pub fn allow(&self, message: &BotMessage) -> Result<(), GateDeny> {
        if !self.enforce {
            return Ok(());
        }
        check_access(&self.store, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entitlements_store::REUSABLE_ALPHA_CODE;

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

    #[test]
    fn link_always_allowed() {
        let store = EntitlementsStore::new_in_memory();
        assert!(check_access(&store, &dm("!link abc", "u")).is_ok());
        assert!(check_access(&store, &group("!link abc", "u", "g1")).is_ok());
    }

    #[test]
    fn unlinked_dm_help_denied() {
        let store = EntitlementsStore::new_in_memory();
        assert_eq!(
            check_access(&store, &dm("!help", "u")).unwrap_err(),
            GateDeny::NeedLink
        );
    }

    #[test]
    fn linked_dm_help_allowed() {
        let store = EntitlementsStore::new_in_memory();
        store.redeem_reusable_alpha("uuid-ada".into()).unwrap();
        assert!(check_access(&store, &dm("!help", "uuid-ada")).is_ok());
    }

    #[test]
    fn presence_without_enable_denies_group_help() {
        let store = EntitlementsStore::new_in_memory();
        store.redeem_reusable_alpha("uuid-ada".into()).unwrap();
        // Alpha present but group not enabled → deny (including for the alpha user).
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

    #[test]
    fn gate_enforce_false_always_allows() {
        let store = EntitlementsStore::new_in_memory();
        let gate = EntitlementGate {
            store,
            enforce: false,
        };
        assert!(gate.allow(&dm("!help", "nobody")).is_ok());
    }

    #[test]
    fn reusable_code_constant_still_usable() {
        assert_eq!(REUSABLE_ALPHA_CODE, "bread-friend");
    }
}
