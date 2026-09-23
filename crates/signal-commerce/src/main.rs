//! Signal commerce sidecar — Stripe Checkout + webhooks → entitlements.enc.

use dstack_client::DstackClient;
use signal_bot::entitlements_store::EntitlementsStore;
use signal_commerce::stripe_client::StripeClient;
use signal_commerce::{create_router, state_from_parts, Config};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() {
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load configuration: {e}");
            std::process::exit(1);
        }
    };

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log.level));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting signal-commerce");

    let stripe = match StripeClient::new(&config.stripe) {
        Ok(c) => c,
        Err(e) => {
            error!("{e:#}");
            std::process::exit(1);
        }
    };
    if config.stripe.secret_key.trim().is_empty() {
        warn!("STRIPE__SECRET_KEY empty — Checkout Session create will fail until configured");
    }
    if config.stripe.webhook_secret.trim().is_empty() {
        warn!("STRIPE__WEBHOOK_SECRET empty — webhook verification will fail");
    }

    let dstack = Arc::new(DstackClient::new(&config.dstack.socket_path));
    let store = EntitlementsStore::open(
        dstack,
        config.entitlements.storage_path.clone(),
        config.entitlements.persist,
        config.legacy_compose_hashes(),
    )
    .await;

    let state = state_from_parts(store, stripe, &config);
    let app = create_router(state, &config.site.cors_origin);

    let addr = SocketAddr::new(
        config
            .server
            .listen_addr
            .parse()
            .unwrap_or_else(|_| [0, 0, 0, 0].into()),
        config.server.port,
    );
    info!("Listening on {addr}");

    let listener = match TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind {addr}: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        error!("Server error: {e}");
        std::process::exit(1);
    }
}
