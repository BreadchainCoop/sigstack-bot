//! Stripe webhook signature verification and event handling.

use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use serde_json::Value;
use sha2::Sha256;
use signal_bot::entitlements_store::{EntitlementStatus, EntitlementsStore};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use subtle::ConstantTimeEq;
use tracing::{info, warn};

use crate::stripe_client::{stripe_id, CheckoutMetadata, PAST_DUE_GRACE_DAYS};

type HmacSha256 = Hmac<Sha256>;

const MAX_SEEN_EVENTS: usize = 10_000;
const SIGNATURE_TOLERANCE_SECS: i64 = 300;

/// In-memory idempotency set for Stripe event ids.
#[derive(Default)]
pub struct SeenEvents {
    inner: Mutex<HashSet<String>>,
}

impl SeenEvents {
    pub fn insert_if_new(&self, event_id: &str) -> bool {
        let mut guard = self.inner.lock().unwrap();
        if guard.contains(event_id) {
            return false;
        }
        if guard.len() >= MAX_SEEN_EVENTS {
            guard.clear();
        }
        guard.insert(event_id.to_string());
        true
    }
}

#[derive(Debug, Deserialize)]
pub struct StripeEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: StripeEventData,
}

#[derive(Debug, Deserialize)]
pub struct StripeEventData {
    pub object: Value,
}

/// Verify `Stripe-Signature` per Stripe docs. Returns Ok(()) when valid.
pub fn verify_signature(
    payload: &[u8],
    sig_header: &str,
    secret: &str,
    now_unix: i64,
) -> Result<(), String> {
    if secret.trim().is_empty() {
        return Err("webhook secret not configured".into());
    }
    let mut timestamp: Option<i64> = None;
    let mut signatures: Vec<&str> = Vec::new();
    for part in sig_header.split(',') {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix("t=") {
            timestamp = rest.parse().ok();
        } else if let Some(rest) = part.strip_prefix("v1=") {
            signatures.push(rest);
        }
    }
    let t = timestamp.ok_or_else(|| "missing t= in Stripe-Signature".to_string())?;
    if (now_unix - t).abs() > SIGNATURE_TOLERANCE_SECS {
        return Err("Stripe-Signature timestamp outside tolerance".into());
    }
    let signed = format!("{t}.{}", String::from_utf8_lossy(payload));
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| "invalid webhook secret".to_string())?;
    mac.update(signed.as_bytes());
    let expected = mac.finalize().into_bytes();
    let expected_hex = hex::encode(expected);

    let ok = signatures.iter().any(|sig| {
        sig.len() == expected_hex.len() && bool::from(sig.as_bytes().ct_eq(expected_hex.as_bytes()))
    });
    if ok {
        Ok(())
    } else {
        Err("Stripe-Signature mismatch".into())
    }
}

pub async fn handle_event(
    store: &Arc<EntitlementsStore>,
    seen: &SeenEvents,
    event: StripeEvent,
) -> Result<(), String> {
    if !seen.insert_if_new(&event.id) {
        info!(event_id = %event.id, "Ignoring duplicate Stripe event");
        return Ok(());
    }

    match event.event_type.as_str() {
        "checkout.session.completed" => on_checkout_completed(store, &event.data.object).await,
        "customer.subscription.updated" => on_subscription_updated(store, &event.data.object).await,
        "customer.subscription.deleted" => on_subscription_deleted(store, &event.data.object).await,
        "invoice.payment_failed" => on_invoice_payment_failed(store, &event.data.object).await,
        other => {
            info!(event_type = other, "Ignoring unhandled Stripe event");
            Ok(())
        }
    }
}

async fn on_checkout_completed(store: &Arc<EntitlementsStore>, obj: &Value) -> Result<(), String> {
    let meta: CheckoutMetadata = obj
        .get("metadata")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| format!("metadata: {e}"))?
        .unwrap_or_default();
    let link_token = meta
        .link_token
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "checkout.session.completed missing metadata.link_token".to_string())?;
    let customer = stripe_id(&obj.get("customer").cloned());
    let subscription = stripe_id(&obj.get("subscription").cloned());

    store
        .with_disk_lock(|s| {
            s.attach_stripe_ids_to_pending(&link_token, customer.clone(), subscription.clone())
        })
        .await??;
    info!(%link_token, "Attached Stripe ids to pending entitlement");
    Ok(())
}

async fn on_subscription_updated(
    store: &Arc<EntitlementsStore>,
    obj: &Value,
) -> Result<(), String> {
    let sub_id = obj
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "subscription missing id".to_string())?;
    let stripe_status = obj
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let (status, expires) = map_subscription_status(stripe_status);
    store
        .with_disk_lock(|s| s.update_by_stripe_subscription(sub_id, status, expires))
        .await??;
    info!(%sub_id, %stripe_status, "Updated entitlement from subscription.updated");
    Ok(())
}

async fn on_subscription_deleted(
    store: &Arc<EntitlementsStore>,
    obj: &Value,
) -> Result<(), String> {
    let sub_id = obj
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "subscription missing id".to_string())?;
    store
        .with_disk_lock(|s| {
            s.update_by_stripe_subscription(sub_id, EntitlementStatus::Canceled, Some(None))
        })
        .await??;
    info!(%sub_id, "Canceled entitlement from subscription.deleted");
    Ok(())
}

async fn on_invoice_payment_failed(
    store: &Arc<EntitlementsStore>,
    obj: &Value,
) -> Result<(), String> {
    let sub_id = match obj.get("subscription") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Object(m)) => m
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        _ => String::new(),
    };
    if sub_id.is_empty() {
        warn!("invoice.payment_failed without subscription; ignoring");
        return Ok(());
    }
    let grace_end = Utc::now() + Duration::days(PAST_DUE_GRACE_DAYS);
    store
        .with_disk_lock(|s| {
            s.update_by_stripe_subscription(
                &sub_id,
                EntitlementStatus::PastDue,
                Some(Some(grace_end)),
            )
        })
        .await??;
    info!(%sub_id, "Marked entitlement past_due with grace");
    Ok(())
}

fn map_subscription_status(
    stripe_status: &str,
) -> (EntitlementStatus, Option<Option<chrono::DateTime<Utc>>>) {
    match stripe_status {
        "active" | "trialing" => (EntitlementStatus::Active, Some(None)),
        "past_due" | "unpaid" => {
            let grace = Utc::now() + Duration::days(PAST_DUE_GRACE_DAYS);
            (EntitlementStatus::PastDue, Some(Some(grace)))
        }
        "canceled" | "incomplete_expired" => (EntitlementStatus::Canceled, Some(None)),
        _ => (EntitlementStatus::PastDue, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_signature_accepts_valid() {
        let secret = "whsec_test_secret";
        let payload = br#"{"id":"evt_1"}"#;
        let t = 1_700_000_000i64;
        let signed = format!("{t}.{}", String::from_utf8_lossy(payload));
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(signed.as_bytes());
        let sig = hex::encode(mac.finalize().into_bytes());
        let header = format!("t={t},v1={sig}");
        assert!(verify_signature(payload, &header, secret, t).is_ok());
    }

    #[test]
    fn verify_signature_rejects_bad() {
        let err = verify_signature(b"{}", "t=1,v1=deadbeef", "whsec_x", 1).unwrap_err();
        assert!(err.contains("mismatch") || err.contains("tolerance"));
    }

    #[test]
    fn seen_events_dedupes() {
        let seen = SeenEvents::default();
        assert!(seen.insert_if_new("evt_a"));
        assert!(!seen.insert_if_new("evt_a"));
    }
}
