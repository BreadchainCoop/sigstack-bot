//! Integration tests for the registration proxy API.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use signal_registration_proxy::{
    api::{create_router_with_rate_limit, AppState, RateLimitState},
    registry::{PhoneNumberRecord, Registry, Store},
    SignalRegistrationClient,
};
use tower::ServiceExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const PHONE: &str = "+14155551234";
const PHONE_ENC: &str = "%2B14155551234";
const SECRET: &str = "owner-secret";

async fn state_with_mock() -> (AppState, MockServer) {
    let mock = MockServer::start().await;
    let signal_client = SignalRegistrationClient::new(mock.uri()).unwrap();
    let state = AppState::new(Registry::new(), Store::memory(), signal_client);
    (state, mock)
}

fn app(state: AppState) -> axum::Router {
    create_router_with_rate_limit(state, RateLimitState::permissive())
}

async fn json_body(response: axum::http::Response<Body>) -> serde_json::Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}

async fn insert_verified(state: &AppState, secret: Option<&str>) {
    let mut record = PhoneNumberRecord::new_pending(
        PHONE.into(),
        secret,
        Some("model-a".into()),
        Some("Be helpful\nMore".into()),
    );
    record.mark_verified();
    let mut registry = state.registry.write().await;
    registry.insert(PHONE.into(), record);
    state.store.save(&registry).await.unwrap();
}

async fn insert_pending(state: &AppState, secret: Option<&str>) {
    let record = PhoneNumberRecord::new_pending(PHONE.into(), secret, None, None);
    let mut registry = state.registry.write().await;
    registry.insert(PHONE.into(), record);
    state.store.save(&registry).await.unwrap();
}

/// Create a test app state with memory-only storage.
fn create_test_state() -> AppState {
    let registry = Registry::new();
    let store = Store::memory();
    let signal_client = SignalRegistrationClient::new("http://localhost:9999").unwrap();
    AppState::new(registry, store, signal_client)
}

#[tokio::test]
async fn test_health_endpoint() {
    let (state, mock) = state_with_mock().await;
    Mock::given(method("GET"))
        .and(path("/v1/health"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock)
        .await;

    let response = app(state)
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json = json_body(response).await;
    assert_eq!(json["status"], "ok");
    assert_eq!(json["registry_count"], 0);
    assert_eq!(json["signal_api_healthy"], true);
}

#[tokio::test]
async fn test_status_not_found() {
    let state = create_test_state();
    let response = app(state)
        .oneshot(
            Request::builder()
                .uri("/v1/status/+14155551234")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_accounts_empty() {
    let state = create_test_state();
    let response = app(state)
        .oneshot(
            Request::builder()
                .uri("/v1/accounts")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json = json_body(response).await;
    assert_eq!(json["total"], 0);
    assert!(json["accounts"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_invalid_phone_number() {
    let state = create_test_state();
    let response = app(state)
        .oneshot(
            Request::builder()
                .uri("/v1/status/invalid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_rate_limiting() {
    let state = create_test_state();
    let rate_limit = RateLimitState::new(1);
    let app = create_router_with_rate_limit(state, rate_limit);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/accounts")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/accounts")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn register_verify_status_flow() {
    let (state, mock) = state_with_mock().await;
    Mock::given(method("POST"))
        .and(path(format!("/v1/register/{PHONE_ENC}")))
        .respond_with(ResponseTemplate::new(201))
        .mount(&mock)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/v1/register/{PHONE_ENC}/verify/111222")))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock)
        .await;

    let router = app(state);

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/register/{PHONE}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "ownership_secret": SECRET, "use_voice": false }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = json_body(response).await;
    assert_eq!(json["status"], "pending");

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/v1/status/{PHONE}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = json_body(response).await;
    assert_eq!(json["status"], "pending");

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/register/{PHONE}/verify/111222"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "ownership_secret": SECRET }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = json_body(response).await;
    assert_eq!(json["status"], "verified");

    let response = router
        .oneshot(
            Request::builder()
                .uri("/v1/accounts")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json = json_body(response).await;
    assert_eq!(json["total"], 1);
}

#[tokio::test]
async fn register_rejects_verified_number() {
    let (state, _mock) = state_with_mock().await;
    insert_verified(&state, Some(SECRET)).await;

    let response = app(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/register/{PHONE}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "ownership_secret": SECRET }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn register_pending_rejects_ownership_mismatch() {
    let (state, mock) = state_with_mock().await;
    insert_pending(&state, Some(SECRET)).await;
    Mock::given(method("POST"))
        .and(path(format!("/v1/register/{PHONE_ENC}")))
        .respond_with(ResponseTemplate::new(201))
        .mount(&mock)
        .await;

    let response = app(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/register/{PHONE}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "ownership_secret": "wrong" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn register_retries_when_signal_already_registered() {
    let (state, mock) = state_with_mock().await;
    Mock::given(method("POST"))
        .and(path(format!("/v1/register/{PHONE_ENC}")))
        .respond_with(ResponseTemplate::new(400).set_body_string("already registered"))
        .up_to_n_times(1)
        .mount(&mock)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/v1/unregister/{PHONE_ENC}")))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/v1/register/{PHONE_ENC}")))
        .respond_with(ResponseTemplate::new(201))
        .mount(&mock)
        .await;

    let response = app(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/register/{PHONE}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "ownership_secret": SECRET }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn unregister_requires_ownership_then_succeeds() {
    let (state, mock) = state_with_mock().await;
    insert_verified(&state, Some(SECRET)).await;
    Mock::given(method("POST"))
        .and(path(format!("/v1/unregister/{PHONE_ENC}")))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock)
        .await;

    let router = app(state);

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/v1/unregister/{PHONE}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "ownership_secret": "nope" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = router
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/v1/unregister/{PHONE}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "ownership_secret": SECRET }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = json_body(response).await;
    assert_eq!(json["status"], "unregistered");
}

#[tokio::test]
async fn adopt_account_success_and_missing_on_signal() {
    let (state, mock) = state_with_mock().await;
    Mock::given(method("GET"))
        .and(path("/v1/accounts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([PHONE])))
        .up_to_n_times(1)
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/accounts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&mock)
        .await;

    let router = app(state);

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/accounts/{PHONE}/adopt"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "ownership_secret": SECRET,
                        "model": "m1",
                        "system_prompt": "hi"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = json_body(response).await;
    assert_eq!(json["status"], "verified");

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/accounts/+15550009999/adopt")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "ownership_secret": SECRET }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn profile_username_and_bot_config() {
    let (state, mock) = state_with_mock().await;
    insert_verified(&state, Some(SECRET)).await;

    Mock::given(method("PUT"))
        .and(path(format!("/v1/profiles/{PHONE_ENC}")))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/v1/accounts/{PHONE_ENC}/username")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "username": "sigstack.54",
            "username_link": "https://signal.me/#eu/x"
        })))
        .mount(&mock)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!("/v1/accounts/{PHONE_ENC}/username")))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/identities/{PHONE_ENC}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "number": PHONE,
            "status": "TRUSTED_VERIFIED",
            "fingerprint": "aa",
            "safety_number": "11111 22222",
            "uuid": "u1"
        }])))
        .mount(&mock)
        .await;

    let router = app(state);

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/v1/profiles/{PHONE}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Bot",
                        "about": "hi",
                        "ownership_secret": SECRET
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/accounts/{PHONE}/username"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "username": "sigstack", "ownership_secret": SECRET }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = json_body(response).await;
    assert_eq!(json["username"], "sigstack.54");

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/v1/bots/{PHONE}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "model": "new-model",
                        "system_prompt": "Updated prompt line",
                        "ownership_secret": SECRET
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/v1/bots/{PHONE}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = json_body(response).await;
    assert_eq!(json["model"], "new-model");

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/bots")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = json_body(response).await;
    assert_eq!(json.as_array().unwrap().len(), 1);
    assert_eq!(json[0]["identity_key"], "11111 22222");

    let response = router
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/v1/accounts/{PHONE}/username"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "ownership_secret": SECRET }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn debug_endpoints() {
    let (state, mock) = state_with_mock().await;
    insert_pending(&state, Some(SECRET)).await;
    Mock::given(method("GET"))
        .and(path("/v1/accounts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([PHONE])))
        .mount(&mock)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/v1/unregister/{PHONE_ENC}")))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock)
        .await;

    let router = app(state);

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/debug/signal-accounts")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = json_body(response).await;
    assert_eq!(json["signal_cli_accounts"][0], PHONE);

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/debug/force-unregister/{PHONE}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = json_body(response).await;
    assert_eq!(json["status"], "unregistered");
}

#[tokio::test]
async fn verify_rejects_wrong_ownership() {
    let (state, _mock) = state_with_mock().await;
    insert_pending(&state, Some(SECRET)).await;

    let response = app(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/register/{PHONE}/verify/123456"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "ownership_secret": "wrong" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn adopt_rejects_already_verified() {
    let (state, _mock) = state_with_mock().await;
    insert_verified(&state, Some(SECRET)).await;

    let response = app(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/accounts/{PHONE}/adopt"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "ownership_secret": SECRET }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn profile_requires_verified_account() {
    let (state, _mock) = state_with_mock().await;
    insert_pending(&state, Some(SECRET)).await;

    let response = app(state)
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/v1/profiles/{PHONE}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "x", "ownership_secret": SECRET }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
