//! Configuration for the commerce sidecar.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub dstack: DstackConfig,
    #[serde(default)]
    pub entitlements: EntitlementsConfig,
    #[serde(default)]
    pub stripe: StripeConfig,
    #[serde(default)]
    pub site: SiteConfig,
    #[serde(default)]
    pub log: LogConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_listen_addr")]
    pub listen_addr: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DstackConfig {
    #[serde(default = "default_dstack_socket")]
    pub socket_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EntitlementsConfig {
    #[serde(default = "default_true")]
    pub persist: bool,
    #[serde(default = "default_entitlements_path")]
    pub storage_path: PathBuf,
    #[serde(default)]
    pub legacy_compose_hash: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StripeConfig {
    /// Secret or restricted key (`sk_…` / `rk_…`). Never log.
    #[serde(default)]
    pub secret_key: String,
    /// Webhook signing secret (`whsec_…`).
    #[serde(default)]
    pub webhook_secret: String,
    #[serde(default)]
    pub price_all_access_3: String,
    #[serde(default)]
    pub price_all_access_10: String,
    #[serde(default = "default_stripe_api_base")]
    pub api_base: String,
    #[serde(default = "default_stripe_api_version")]
    pub api_version: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SiteConfig {
    /// Public Pages origin + base path, no trailing slash.
    /// Example: `https://breadchaincoop.github.io/sigstack-bot/sigstack`
    #[serde(default = "default_site_public_base")]
    pub public_base_url: String,
    /// Allowed CORS origin for checkout create (Pages origin).
    #[serde(default = "default_cors_origin")]
    pub cors_origin: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen_addr: default_listen_addr(),
            port: default_port(),
        }
    }
}

impl Default for DstackConfig {
    fn default() -> Self {
        Self {
            socket_path: default_dstack_socket(),
        }
    }
}

impl Default for EntitlementsConfig {
    fn default() -> Self {
        Self {
            persist: true,
            storage_path: default_entitlements_path(),
            legacy_compose_hash: String::new(),
        }
    }
}

impl Default for StripeConfig {
    fn default() -> Self {
        Self {
            secret_key: String::new(),
            webhook_secret: String::new(),
            price_all_access_3: String::new(),
            price_all_access_10: String::new(),
            api_base: default_stripe_api_base(),
            api_version: default_stripe_api_version(),
        }
    }
}

impl Default for SiteConfig {
    fn default() -> Self {
        Self {
            public_base_url: default_site_public_base(),
            cors_origin: default_cors_origin(),
        }
    }
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

fn default_listen_addr() -> String {
    "0.0.0.0".into()
}

fn default_port() -> u16 {
    8082
}

fn default_dstack_socket() -> String {
    "/var/run/dstack.sock".into()
}

fn default_true() -> bool {
    true
}

fn default_entitlements_path() -> PathBuf {
    PathBuf::from("/data/entitlements.enc")
}

fn default_stripe_api_base() -> String {
    "https://api.stripe.com".into()
}

fn default_stripe_api_version() -> String {
    "2026-04-22.dahlia".into()
}

fn default_site_public_base() -> String {
    "https://breadchaincoop.github.io/sigstack-bot/sigstack".into()
}

fn default_cors_origin() -> String {
    "https://breadchaincoop.github.io".into()
}

fn default_log_level() -> String {
    "info".into()
}

impl Config {
    pub fn load() -> Result<Self> {
        dotenvy::dotenv().ok();
        let config = config::Config::builder()
            .add_source(
                config::Environment::default()
                    .separator("__")
                    .try_parsing(false),
            )
            .build()
            .context("Failed to build configuration")?;
        config
            .try_deserialize()
            .context("Failed to deserialize configuration")
    }

    pub fn legacy_compose_hashes(&self) -> Vec<String> {
        self.entitlements
            .legacy_compose_hash
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let s = ServerConfig::default();
        assert_eq!(s.port, 8082);
        let stripe = StripeConfig::default();
        assert_eq!(stripe.api_version, "2026-04-22.dahlia");
    }
}
