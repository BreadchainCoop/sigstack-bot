//! Ops CLI: mint pending alpha entitlement codes into the encrypted store.
//!
//! ```bash
//! # On CVM / with dstack + volume:
//! cargo run -p signal-bot --bin mint-alpha -- --count 5
//!
//! # Env (defaults match compose):
//! #   ENTITLEMENTS__STORAGE_PATH=/data/entitlements.enc
//! #   DSTACK__SOCKET_PATH=/var/run/dstack.sock
//! #   ENTITLEMENTS__LEGACY_COMPOSE_HASH=  (optional)
//! ```

use anyhow::{bail, Context, Result};
use chrono::Utc;
use dstack_client::DstackClient;
use signal_bot::entitlements_store::EntitlementsStore;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn legacy_hashes(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args: Vec<String> = env::args().skip(1).collect();
    let mut count: usize = 1;
    let mut days: i64 = 90;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--count" => {
                i += 1;
                count = args
                    .get(i)
                    .context("--count needs a value")?
                    .parse()
                    .context("invalid --count")?;
            }
            "--days" => {
                i += 1;
                days = args
                    .get(i)
                    .context("--days needs a value")?
                    .parse()
                    .context("invalid --days")?;
            }
            "--help" | "-h" => {
                eprintln!(
                    "Usage: mint-alpha [--count N] [--days 90]\n\
                     Env: ENTITLEMENTS__STORAGE_PATH (default /data/entitlements.enc)\n\
                          DSTACK__SOCKET_PATH (default /var/run/dstack.sock)\n\
                          ENTITLEMENTS__LEGACY_COMPOSE_HASH (optional)"
                );
                return Ok(());
            }
            other => bail!("unknown argument: {other}"),
        }
        i += 1;
    }

    let storage_path =
        env::var("ENTITLEMENTS__STORAGE_PATH").unwrap_or_else(|_| "/data/entitlements.enc".into());
    let socket = env::var("DSTACK__SOCKET_PATH").unwrap_or_else(|_| "/var/run/dstack.sock".into());
    let legacy = env::var("ENTITLEMENTS__LEGACY_COMPOSE_HASH").unwrap_or_default();

    let dstack = Arc::new(DstackClient::new(&socket));
    let store = EntitlementsStore::open(
        dstack,
        PathBuf::from(&storage_path),
        true,
        legacy_hashes(&legacy),
    )
    .await;

    let codes = store
        .mint_alpha_codes(count, days)
        .map_err(|e| anyhow::anyhow!(e))?;
    store.flush().await.map_err(|e| anyhow::anyhow!(e))?;

    info!(
        count = codes.len(),
        days,
        path = %storage_path,
        "minted alpha codes at {}",
        Utc::now()
    );

    println!("# alpha codes (single-use, {days}-day expiry) — share /alpha?code=<token>");
    for code in &codes {
        println!("{code}");
    }
    Ok(())
}
