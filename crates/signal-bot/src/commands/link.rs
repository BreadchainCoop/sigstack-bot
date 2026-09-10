//! `!link <code>` and `!enable-sigstack` — bind entitlements and unlock groups (alpha MVP).

use crate::commands::CommandHandler;
use crate::entitlements_store::{
    is_reusable_alpha_code, EntitlementSource, EntitlementsStore, PlanSku, RedeemReuseError,
};
use crate::error::AppResult;
use async_trait::async_trait;
use chrono::Utc;
use signal_bot_core::{starts_with_word, strip_word_prefix};
use signal_client::BotMessage;
use std::sync::Arc;

const LINK_USAGE: &str = "Usage: !link <code>";
const ENABLE_USAGE: &str =
    "Usage: !enable-sigstack (run in the Signal group after inviting this bot)";
const LINK_GROUP_WARN: &str =
    "Tip: prefer DMing this bot with !link so your code is not visible in the group.";
const ALREADY_ACTIVE_MSG: &str =
    "You already have an active alpha entitlement linked to this account.";
const ENABLE_SUCCESS: &str =
    "Sigstack enabled in this group. Anyone here can use !help and product commands.";

pub struct LinkHandler {
    entitlements: Arc<EntitlementsStore>,
}

impl LinkHandler {
    pub fn new(entitlements: Arc<EntitlementsStore>) -> Self {
        Self { entitlements }
    }

    fn owner_key(message: &BotMessage) -> String {
        // Prefer envelope source (often UUID); phone is already in source when UUID absent.
        message.source.trim().to_string()
    }

    fn parse_link_code(text: &str) -> Option<&str> {
        Some(strip_word_prefix(text, "!link")?.trim())
    }

    fn linked_reply(
        bound_sku: PlanSku,
        bound_source: EntitlementSource,
        expires_at: Option<chrono::DateTime<Utc>>,
        is_group: bool,
    ) -> String {
        let mut reply = format!(
            "Linked. Plan: {:?} ({:?}). You have access through {}.",
            bound_sku,
            bound_source,
            expires_at
                .map(|t| t.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "the plan end date".into())
        );
        if bound_sku.is_group_claimable() {
            reply.push_str(
                "\n\nIn a group, run !enable-sigstack so everyone in that chat can use Sigstack.",
            );
        }
        if is_group {
            reply = format!("{LINK_GROUP_WARN}\n\n{reply}");
        }
        reply
    }

    async fn handle_link(&self, message: &BotMessage) -> AppResult<String> {
        let Some(code) = Self::parse_link_code(&message.text) else {
            return Ok(LINK_USAGE.into());
        };
        if code.is_empty() {
            return Ok(format!("Code cannot be empty.\n{LINK_USAGE}"));
        }

        let owner = Self::owner_key(message);
        if owner.is_empty() {
            return Ok("Could not determine your Signal identity. Try again from Signal.".into());
        }

        if is_reusable_alpha_code(code) {
            return match self.entitlements.redeem_reusable_alpha(owner) {
                Ok(bound) => Ok(Self::linked_reply(
                    bound.plan_sku,
                    bound.source,
                    bound.expires_at,
                    message.is_group,
                )),
                Err(RedeemReuseError::AlreadyActive) => Ok(ALREADY_ACTIVE_MSG.into()),
                Err(RedeemReuseError::EmptyOwner) => {
                    Ok("Could not determine your Signal identity. Try again from Signal.".into())
                }
            };
        }

        let pending = match self.entitlements.get_pending(code) {
            Some(p) => p,
            None => {
                let owned = self.entitlements.get_individual(&owner);
                if owned.iter().any(|r| {
                    r.plan_sku == PlanSku::BundleAllAlpha
                        && r.source == EntitlementSource::Alpha
                        && r.is_granting_at(Utc::now())
                }) {
                    return Ok(ALREADY_ACTIVE_MSG.into());
                }
                return Ok(
                    "Unknown or already-used code. Check the code and try again, or ask for a new alpha code."
                        .into(),
                );
            }
        };

        if !pending.is_granting_at(Utc::now()) {
            let _ = self.entitlements.expire_due(Utc::now());
            return Ok("That code has expired. Ask for a new alpha code.".into());
        }

        match self.entitlements.bind_link_token(code, owner.clone()) {
            Ok(bound) => Ok(Self::linked_reply(
                bound.plan_sku,
                bound.source,
                bound.expires_at,
                message.is_group,
            )),
            Err(e) => Ok(format!("Could not link that code: {e}")),
        }
    }

    async fn handle_enable_sigstack(&self, message: &BotMessage) -> AppResult<String> {
        let Some(group_id) = message.group_id.as_deref() else {
            return Ok(ENABLE_USAGE.into());
        };
        let owner = Self::owner_key(message);
        if owner.is_empty() {
            return Ok("Could not determine your Signal identity.".into());
        }

        if !self.entitlements.has_active_individual(&owner, Utc::now()) {
            return Ok(
                "No entitlement found. Link an alpha (or group) plan with !link <code> first."
                    .into(),
            );
        }

        match self
            .entitlements
            .enable_sigstack(&owner, group_id.to_string())
        {
            Ok(_) => Ok(ENABLE_SUCCESS.into()),
            Err(e) => Ok(format!("Could not enable Sigstack in this group: {e}")),
        }
    }
}

#[async_trait]
impl CommandHandler for LinkHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        starts_with_word(&message.text, "!link")
            || starts_with_word(&message.text, "!enable-sigstack")
    }

    fn label(&self) -> &'static str {
        "link"
    }

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        if starts_with_word(&message.text, "!enable-sigstack") {
            return self.handle_enable_sigstack(message).await;
        }
        if starts_with_word(&message.text, "!link") {
            return self.handle_link(message).await;
        }
        Ok(LINK_USAGE.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entitlements_store::REUSABLE_ALPHA_CODE;
    use chrono::Duration;

    fn dm(text: &str, source: &str) -> BotMessage {
        BotMessage {
            source: source.into(),
            source_number: Some("+15551234567".into()),
            source_name: Some("Ada".into()),
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

    fn group_msg(text: &str, source: &str, group_id: &str) -> BotMessage {
        let mut m = dm(text, source);
        m.is_group = true;
        m.group_id = Some(group_id.into());
        m
    }

    #[tokio::test]
    async fn link_binds_pending_alpha() {
        let store = EntitlementsStore::new_in_memory();
        let codes = store.mint_alpha_codes(1, 90).unwrap();
        let handler = LinkHandler::new(store.clone());

        let reply = handler
            .execute(&dm(&format!("!link {}", codes[0]), "uuid-ada"))
            .await
            .unwrap();
        assert!(reply.contains("Linked"), "{reply}");
        assert!(reply.contains("!enable-sigstack"), "{reply}");
        assert_eq!(store.get_individual("uuid-ada").len(), 1);
        assert!(store.get_pending(&codes[0]).is_none());
    }

    #[tokio::test]
    async fn link_rejects_unknown_code() {
        let store = EntitlementsStore::new_in_memory();
        let handler = LinkHandler::new(store);
        let reply = handler
            .execute(&dm("!link not-a-real-code", "uuid-ada"))
            .await
            .unwrap();
        assert!(reply.contains("Unknown"), "{reply}");
    }

    #[tokio::test]
    async fn link_rejects_expired() {
        let store = EntitlementsStore::new_in_memory();
        let past = Utc::now() - Duration::days(1);
        store
            .create_pending(
                "expired-code".into(),
                PlanSku::BundleAllAlpha,
                EntitlementSource::Alpha,
                Some(past),
                None,
                None,
            )
            .unwrap();
        let handler = LinkHandler::new(store);
        let reply = handler
            .execute(&dm("!link expired-code", "uuid-ada"))
            .await
            .unwrap();
        assert!(reply.to_lowercase().contains("expired"), "{reply}");
    }

    #[tokio::test]
    async fn enable_sigstack_after_link() {
        let store = EntitlementsStore::new_in_memory();
        let codes = store.mint_alpha_codes(1, 90).unwrap();
        let handler = LinkHandler::new(store.clone());
        handler
            .execute(&dm(&format!("!link {}", codes[0]), "uuid-ada"))
            .await
            .unwrap();

        let reply = handler
            .execute(&group_msg("!enable-sigstack", "uuid-ada", "group.main"))
            .await
            .unwrap();
        assert!(reply.contains("enabled"), "{reply}");
        assert_eq!(store.get_group("group.main").len(), 1);
    }

    #[tokio::test]
    async fn enable_sigstack_multiple_groups() {
        let store = EntitlementsStore::new_in_memory();
        let handler = LinkHandler::new(store.clone());
        handler
            .execute(&dm(&format!("!link {REUSABLE_ALPHA_CODE}"), "uuid-ada"))
            .await
            .unwrap();

        handler
            .execute(&group_msg("!enable-sigstack", "uuid-ada", "group.a"))
            .await
            .unwrap();
        handler
            .execute(&group_msg("!enable-sigstack", "uuid-ada", "group.b"))
            .await
            .unwrap();
        assert_eq!(store.get_group("group.a").len(), 1);
        assert_eq!(store.get_group("group.b").len(), 1);
    }

    #[tokio::test]
    async fn enable_sigstack_requires_link() {
        let store = EntitlementsStore::new_in_memory();
        let handler = LinkHandler::new(store);
        let reply = handler
            .execute(&group_msg("!enable-sigstack", "uuid-bob", "group.main"))
            .await
            .unwrap();
        assert!(reply.contains("!link"), "{reply}");
    }

    #[tokio::test]
    async fn reusable_bread_grants_two_owners() {
        let store = EntitlementsStore::new_in_memory();
        let handler = LinkHandler::new(store.clone());

        let a = handler
            .execute(&dm(&format!("!link {REUSABLE_ALPHA_CODE}"), "uuid-a"))
            .await
            .unwrap();
        assert!(a.contains("Linked"), "{a}");
        let b = handler.execute(&dm("!link Bread", "uuid-b")).await.unwrap();
        assert!(b.contains("Linked"), "{b}");
        assert_eq!(store.get_individual("uuid-a").len(), 1);
        assert_eq!(store.get_individual("uuid-b").len(), 1);

        let again = handler
            .execute(&dm(&format!("!link {REUSABLE_ALPHA_CODE}"), "uuid-a"))
            .await
            .unwrap();
        assert!(again.contains("already have an active alpha"), "{again}");
    }

    #[tokio::test]
    async fn minted_single_use_still_consumed() {
        let store = EntitlementsStore::new_in_memory();
        let codes = store.mint_alpha_codes(1, 90).unwrap();
        let handler = LinkHandler::new(store.clone());
        handler
            .execute(&dm(&format!("!link {}", codes[0]), "uuid-one"))
            .await
            .unwrap();
        assert!(store.get_pending(&codes[0]).is_none());
        let second = handler
            .execute(&dm(&format!("!link {}", codes[0]), "uuid-two"))
            .await
            .unwrap();
        assert!(second.contains("Unknown"), "{second}");
    }

    #[test]
    fn matches_link_and_enable() {
        let handler = LinkHandler::new(EntitlementsStore::new_in_memory());
        assert!(handler.matches(&dm("!link abc", "u")));
        assert!(handler.matches(&dm("!enable-sigstack", "u")));
        assert!(!handler.matches(&dm("!claim-group", "u")));
        assert!(!handler.matches(&dm("!help", "u")));
    }
}
