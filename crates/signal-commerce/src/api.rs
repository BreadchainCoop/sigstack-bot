//! HTTP routes for checkout + Stripe webhooks.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{Duration, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use signal_bot::entitlements_store::{EntitlementSource, EntitlementsStore};
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

use crate::config::{Config, SiteConfig, StripeConfig};
use crate::sku::{parse_plan_sku, plan_sku_str, price_id_for};
use crate::stripe_client::{StripeClient, PENDING_TTL_HOURS};
use crate::webhook::{handle_event, verify_signature, SeenEvents, StripeEvent};

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<EntitlementsStore>,
    pub stripe: StripeClient,
    pub stripe_cfg: StripeConfig,
    pub site: SiteConfig,
    pub webhook_secret: String,
    pub seen: Arc<SeenEvents>,
}

pub fn create_router(state: AppState, cors_origin: &str) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::exact(cors_origin.parse().unwrap_or_else(
            |_| {
                "https://breadchaincoop.github.io"
                    .parse()
                    .expect("static origin")
            },
        )))
        .allow_methods([axum::http::Method::POST, axum::http::Method::OPTIONS])
        .allow_headers([axum::http::header::CONTENT_TYPE, axum::http::header::ACCEPT]);

    Router::new()
        .route("/health", get(health))
        .route("/v1/checkout/sessions", post(create_checkout_session))
        .route("/v1/webhooks/stripe", post(stripe_webhook))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok", "service": "signal-commerce" }))
}

#[derive(Debug, Deserialize)]
pub struct CreateCheckoutRequest {
    pub plan_sku: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCheckoutResponse {
    pub url: String,
    pub plan_sku: String,
    /// Present so clients can show success copy without waiting on redirect query.
    pub link_token: String,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
}

async fn create_checkout_session(
    State(state): State<AppState>,
    Json(body): Json<CreateCheckoutRequest>,
) -> impl IntoResponse {
    let Some(sku) = parse_plan_sku(&body.plan_sku) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "unknown plan_sku; expected all-access-3 or all-access-10".into(),
            }),
        )
            .into_response();
    };

    let price_id = match price_id_for(sku, &state.stripe_cfg) {
        Ok(id) => id.to_string(),
        Err(e) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorBody { error: e }),
            )
                .into_response();
        }
    };

    let link_token = new_link_token();
    let plan = plan_sku_str(sku).to_string();
    let expires_at = Utc::now() + Duration::hours(PENDING_TTL_HOURS);

    let pending_result = state
        .store
        .with_disk_lock(|s| {
            s.create_pending(
                link_token.clone(),
                sku,
                EntitlementSource::Stripe,
                Some(expires_at),
                None,
                None,
            )
        })
        .await;

    match pending_result {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => {
            warn!("create_pending rejected: {e}");
            return (
                StatusCode::CONFLICT,
                Json(ErrorBody {
                    error: "could not reserve link token".into(),
                }),
            )
                .into_response();
        }
        Err(e) => {
            warn!("create_pending failed: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: "failed to create pending entitlement".into(),
                }),
            )
                .into_response();
        }
    }

    let base = state.site.public_base_url.trim_end_matches('/');
    let success_url = format!("{base}/checkout/success/?code={link_token}&plan={plan}");
    let cancel_url = format!("{base}/checkout/cancel/");

    match state
        .stripe
        .create_checkout_session(&price_id, &plan, &link_token, &success_url, &cancel_url)
        .await
    {
        Ok(session) => {
            let Some(url) = session.url.filter(|u| !u.is_empty()) else {
                return (
                    StatusCode::BAD_GATEWAY,
                    Json(ErrorBody {
                        error: "stripe session missing url".into(),
                    }),
                )
                    .into_response();
            };
            info!(%plan, session_id = %session.id, "Created Checkout Session");
            (
                StatusCode::OK,
                Json(CreateCheckoutResponse {
                    url,
                    plan_sku: plan,
                    link_token,
                }),
            )
                .into_response()
        }
        Err(e) => {
            warn!("stripe checkout create failed: {e:#}");
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorBody {
                    error: "stripe checkout session create failed".into(),
                }),
            )
                .into_response()
        }
    }
}

async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let sig = headers
        .get("Stripe-Signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let now = Utc::now().timestamp();
    if let Err(e) = verify_signature(&body, sig, &state.webhook_secret, now) {
        warn!("webhook signature rejected: {e}");
        return (StatusCode::BAD_REQUEST, Json(ErrorBody { error: e })).into_response();
    }

    let event: StripeEvent = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorBody {
                    error: format!("invalid event json: {e}"),
                }),
            )
                .into_response();
        }
    };

    match handle_event(&state.store, &state.seen, event).await {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => {
            warn!("webhook handler error: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody { error: e }),
            )
                .into_response()
        }
    }
}

fn new_link_token() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Build [`AppState`] from loaded config (tests + main).
pub fn state_from_parts(
    store: Arc<EntitlementsStore>,
    stripe: StripeClient,
    cfg: &Config,
) -> AppState {
    AppState {
        store,
        stripe,
        stripe_cfg: cfg.stripe.clone(),
        site: cfg.site.clone(),
        webhook_secret: cfg.stripe.webhook_secret.clone(),
        seen: Arc::new(SeenEvents::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use dstack_client::DstackClient;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    use signal_bot::entitlements_store::PlanSku;
    use tower::ServiceExt;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    type HmacSha256 = Hmac<Sha256>;

    async fn test_store(path: std::path::PathBuf) -> Arc<EntitlementsStore> {
        EntitlementsStore::with_test_key(DstackClient::new("/x"), path, [3u8; 32]).await
    }

    fn test_cfg(api_base: Option<String>) -> Config {
        Config {
            stripe: StripeConfig {
                secret_key: "sk_test_x".into(),
                webhook_secret: "whsec_test".into(),
                price_all_access_3: "price_3".into(),
                price_all_access_10: "price_10".into(),
                api_base: api_base.unwrap_or_else(|| "https://api.stripe.com".into()),
                ..StripeConfig::default()
            },
            ..Config::default()
        }
    }

    #[tokio::test]
    async fn health_ok() {
        let dir = tempfile::tempdir().unwrap();
        let store = test_store(dir.path().join("e.enc")).await;
        let cfg = test_cfg(None);
        let stripe = StripeClient::new(&cfg.stripe).unwrap();
        let app = create_router(state_from_parts(store, stripe, &cfg), &cfg.site.cors_origin);
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn checkout_unknown_sku() {
        let dir = tempfile::tempdir().unwrap();
        let store = test_store(dir.path().join("e.enc")).await;
        let cfg = test_cfg(None);
        let stripe = StripeClient::new(&cfg.stripe).unwrap();
        let app = create_router(state_from_parts(store, stripe, &cfg), &cfg.site.cors_origin);
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/checkout/sessions")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"plan_sku":"nope"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn checkout_creates_pending_and_returns_url() {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/checkout/sessions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cs_test_1",
                "url": "https://checkout.stripe.com/c/pay/cs_test_1",
                "metadata": {}
            })))
            .mount(&mock)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let store = test_store(dir.path().join("e.enc")).await;
        let cfg = test_cfg(Some(mock.uri()));
        let stripe = StripeClient::new(&cfg.stripe).unwrap();
        let app = create_router(
            state_from_parts(store.clone(), stripe, &cfg),
            &cfg.site.cors_origin,
        );
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/checkout/sessions")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"plan_sku":"all-access-3"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(res.into_body(), 1024 * 64)
            .await
            .unwrap();
        let parsed: CreateCheckoutResponse = serde_json::from_slice(&bytes).unwrap();
        assert!(parsed.url.contains("checkout.stripe.com"));
        assert_eq!(parsed.plan_sku, "all-access-3");
        let pending = store.get_pending(&parsed.link_token).unwrap();
        assert_eq!(pending.plan_sku, PlanSku::AllAccess3);
        assert_eq!(pending.source, EntitlementSource::Stripe);
    }

    #[tokio::test]
    async fn webhook_rejects_bad_signature() {
        let dir = tempfile::tempdir().unwrap();
        let store = test_store(dir.path().join("e.enc")).await;
        let cfg = test_cfg(None);
        let stripe = StripeClient::new(&cfg.stripe).unwrap();
        let app = create_router(state_from_parts(store, stripe, &cfg), &cfg.site.cors_origin);
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/webhooks/stripe")
                    .header("Stripe-Signature", "t=1,v1=dead")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn webhook_checkout_completed_attaches_ids() {
        let dir = tempfile::tempdir().unwrap();
        let store = test_store(dir.path().join("e.enc")).await;
        store
            .create_pending(
                "tok123".into(),
                PlanSku::AllAccess3,
                EntitlementSource::Stripe,
                None,
                None,
                None,
            )
            .unwrap();
        store.flush().await.unwrap();

        let cfg = test_cfg(None);
        let stripe = StripeClient::new(&cfg.stripe).unwrap();
        let app = create_router(
            state_from_parts(store.clone(), stripe, &cfg),
            &cfg.site.cors_origin,
        );

        let payload = serde_json::json!({
            "id": "evt_test_1",
            "type": "checkout.session.completed",
            "data": {
                "object": {
                    "id": "cs_1",
                    "customer": "cus_1",
                    "subscription": "sub_1",
                    "metadata": { "link_token": "tok123", "plan_sku": "all-access-3" }
                }
            }
        });
        let body = serde_json::to_vec(&payload).unwrap();
        let t = Utc::now().timestamp();
        let signed = format!("{t}.{}", String::from_utf8_lossy(&body));
        let mut mac = HmacSha256::new_from_slice(b"whsec_test").unwrap();
        mac.update(signed.as_bytes());
        let sig = hex::encode(mac.finalize().into_bytes());
        let header = format!("t={t},v1={sig}");

        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/webhooks/stripe")
                    .header("Stripe-Signature", header)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        store.reload().await.unwrap();
        let pending = store.get_pending("tok123").unwrap();
        assert_eq!(pending.stripe_customer_id.as_deref(), Some("cus_1"));
        assert_eq!(pending.stripe_subscription_id.as_deref(), Some("sub_1"));
    }
}
