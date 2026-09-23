//! Minimal Stripe Checkout Sessions client (reqwest).

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use crate::config::StripeConfig;

pub const PENDING_TTL_HOURS: i64 = 48;
pub const PAST_DUE_GRACE_DAYS: i64 = 7;

#[derive(Clone)]
pub struct StripeClient {
    http: Client,
    secret_key: String,
    api_base: String,
    api_version: String,
}

#[derive(Debug, Deserialize)]
pub struct CheckoutSession {
    pub id: String,
    pub url: Option<String>,
    pub customer: Option<Value>,
    pub subscription: Option<Value>,
    pub metadata: Option<CheckoutMetadata>,
    pub payment_status: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct CheckoutMetadata {
    pub plan_sku: Option<String>,
    pub link_token: Option<String>,
}

impl StripeClient {
    pub fn new(cfg: &StripeConfig) -> Result<Self> {
        Ok(Self {
            http: Client::new(),
            secret_key: cfg.secret_key.clone(),
            api_base: cfg.api_base.trim_end_matches('/').to_string(),
            api_version: cfg.api_version.clone(),
        })
    }

    /// Create a subscription Checkout Session.
    pub async fn create_checkout_session(
        &self,
        price_id: &str,
        plan_sku: &str,
        link_token: &str,
        success_url: &str,
        cancel_url: &str,
    ) -> Result<CheckoutSession> {
        if self.secret_key.trim().is_empty() {
            return Err(anyhow!("STRIPE__SECRET_KEY is not configured"));
        }
        let url = format!("{}/v1/checkout/sessions", self.api_base);
        let params = [
            ("mode", "subscription"),
            ("line_items[0][price]", price_id),
            ("line_items[0][quantity]", "1"),
            ("success_url", success_url),
            ("cancel_url", cancel_url),
            ("metadata[plan_sku]", plan_sku),
            ("metadata[link_token]", link_token),
            ("subscription_data[metadata][plan_sku]", plan_sku),
            ("subscription_data[metadata][link_token]", link_token),
            ("client_reference_id", link_token),
        ];

        let res = self
            .http
            .post(&url)
            .basic_auth(&self.secret_key, None::<&str>)
            .header("Stripe-Version", &self.api_version)
            .form(&params)
            .send()
            .await
            .context("stripe checkout.sessions.create request")?;

        let status = res.status();
        let body = res.text().await.context("stripe response body")?;
        if !status.is_success() {
            return Err(anyhow!(
                "stripe checkout.sessions.create failed ({status}): {}",
                redact_secrets(&body)
            ));
        }
        serde_json::from_str(&body).context("parse checkout session")
    }
}

fn redact_secrets(s: &str) -> String {
    // Avoid echoing accidental key material in error paths.
    if s.contains("sk_") || s.contains("rk_") || s.contains("whsec_") {
        "[redacted stripe error body]".into()
    } else {
        s.chars().take(500).collect()
    }
}

/// Extract a Stripe id string from a JSON value that may be a string or expanded object.
pub fn stripe_id(value: &Option<Value>) -> Option<String> {
    match value {
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::Object(map)) => map
            .get("id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn stripe_id_from_string_or_object() {
        assert_eq!(
            stripe_id(&Some(Value::String("cus_1".into()))).as_deref(),
            Some("cus_1")
        );
        assert_eq!(
            stripe_id(&Some(json!({"id": "sub_1"}))).as_deref(),
            Some("sub_1")
        );
        assert!(stripe_id(&None).is_none());
    }
}
