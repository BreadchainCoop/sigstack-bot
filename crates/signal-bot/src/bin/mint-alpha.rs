//! Ops CLI: mint, list, and revoke alpha entitlement codes.
//!
//! ```bash
//! # On CVM / with dstack + volume (never commit codes):
//! cargo run -p signal-bot --bin mint-alpha -- mint --count 5
//! cargo run -p signal-bot --bin mint-alpha -- list
//! cargo run -p signal-bot --bin mint-alpha -- revoke <code|record-id|owner-uuid>
//!
//! # Back-compat (same as `mint`):
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

fn usage() {
    eprintln!(
        "Usage:\n\
         \tmint-alpha mint [--count N] [--days 90]\n\
         \tmint-alpha list\n\
         \tmint-alpha revoke <code|record-id|owner-uuid>\n\
         \tmint-alpha --count N [--days 90]   (same as mint)\n\
         Env: ENTITLEMENTS__STORAGE_PATH (default /data/entitlements.enc)\n\
              DSTACK__SOCKET_PATH (default /var/run/dstack.sock)\n\
              ENTITLEMENTS__LEGACY_COMPOSE_HASH (optional)"
    );
}

async fn open_store() -> Result<Arc<EntitlementsStore>> {
    let storage_path =
        env::var("ENTITLEMENTS__STORAGE_PATH").unwrap_or_else(|_| "/data/entitlements.enc".into());
    let socket = env::var("DSTACK__SOCKET_PATH").unwrap_or_else(|_| "/var/run/dstack.sock".into());
    let legacy = env::var("ENTITLEMENTS__LEGACY_COMPOSE_HASH").unwrap_or_default();

    let dstack = Arc::new(DstackClient::new(&socket));
    Ok(EntitlementsStore::open(
        dstack,
        PathBuf::from(&storage_path),
        true,
        legacy_hashes(&legacy),
    )
    .await)
}

fn parse_mint_flags(args: &[String]) -> Result<(usize, i64)> {
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
            other => bail!("unknown mint argument: {other}"),
        }
        i += 1;
    }
    Ok((count, days))
}

async fn cmd_mint(args: &[String]) -> Result<()> {
    let (count, days) = parse_mint_flags(args)?;
    let storage_path =
        env::var("ENTITLEMENTS__STORAGE_PATH").unwrap_or_else(|_| "/data/entitlements.enc".into());
    let store = open_store().await?;
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

async fn cmd_list() -> Result<()> {
    let store = open_store().await?;
    let rows = store.list_alpha();
    println!("state\tstatus\texpires_at\towner\trecord_id\tcode");
    for row in rows {
        let expires = row
            .expires_at
            .map(|t| t.to_rfc3339())
            .unwrap_or_else(|| "-".into());
        let owner = row.owner_uuid.as_deref().unwrap_or("-");
        let code = row.code.as_deref().unwrap_or("-");
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}",
            row.state,
            row.status.as_str(),
            expires,
            owner,
            row.record_id,
            code
        );
    }
    Ok(())
}

async fn cmd_revoke(target: &str) -> Result<()> {
    let store = open_store().await?;
    let affected = store.revoke_alpha(target).map_err(|e| anyhow::anyhow!(e))?;
    store.flush().await.map_err(|e| anyhow::anyhow!(e))?;
    info!(n = affected.len(), target, "revoked alpha");
    for rec in &affected {
        let owner = rec.owner_uuid.as_deref().unwrap_or("-");
        println!(
            "revoked\t{}\t{}\t{}\t{}",
            rec.status.as_str(),
            owner,
            rec.id,
            rec.link_token.as_deref().unwrap_or("-")
        );
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || matches!(args[0].as_str(), "--help" | "-h") {
        usage();
        return Ok(());
    }

    // Back-compat: `mint-alpha --count N` (no subcommand).
    if args[0].starts_with("--") {
        return cmd_mint(&args).await;
    }

    match args[0].as_str() {
        "mint" => cmd_mint(&args[1..]).await,
        "list" => {
            if args.len() > 1 {
                bail!("list takes no arguments");
            }
            cmd_list().await
        }
        "revoke" => {
            let target = args
                .get(1)
                .context("revoke needs <code|record-id|owner-uuid>")?;
            if args.len() > 2 {
                bail!("revoke takes exactly one argument");
            }
            cmd_revoke(target).await
        }
        other => {
            usage();
            bail!("unknown command: {other}");
        }
    }
}
