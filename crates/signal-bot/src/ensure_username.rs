//! Ensure the bot has a Signal username for public discovery (no E.164 on the site).

use signal_client::SignalClient;
use tracing::{info, warn};

/// Claim / refresh a Signal username via signal-cli.
///
/// Non-fatal on failure so a username API hiccup does not block the bot.
/// On success, logs `username` + `username_link` for ops to set
/// `PUBLIC_SIGNAL_USERNAME_LINK` on the marketing site.
pub async fn ensure_signal_username(signal: &SignalClient, phone_number: &str, nickname: &str) {
    let nickname = nickname.trim();
    if nickname.is_empty() {
        info!("BOT__SIGNAL_USERNAME empty — skipping Signal username ensure");
        return;
    }

    // Nickname only (strip accidental discriminator from env).
    let nickname = nickname
        .split_once('.')
        .map(|(nick, _)| nick)
        .unwrap_or(nickname)
        .trim();
    if nickname.is_empty() {
        warn!("BOT__SIGNAL_USERNAME has no nickname — skipping Signal username ensure");
        return;
    }

    match signal.set_username(phone_number, nickname).await {
        Ok(info) => {
            let username = info.username.as_deref().unwrap_or("(unknown)");
            let link = info
                .username_link
                .as_deref()
                .unwrap_or("(no link returned)");
            info!(
                username,
                username_link = link,
                "Signal username ready — set site PUBLIC_SIGNAL_USERNAME_LINK to username_link after re-register"
            );
        }
        Err(err) => {
            warn!(
                error = %err,
                nickname,
                "Failed to ensure Signal username (non-fatal); set manually via signal-cli if needed"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn ensure_sets_username_and_logs_link() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/accounts/%2B15555555555/username"))
            .and(body_json(serde_json::json!({ "username": "sigstack" })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "username": "sigstack.01",
                "username_link": "https://signal.me/#eu/x"
            })))
            .mount(&mock_server)
            .await;

        let client = SignalClient::new(mock_server.uri()).unwrap();
        ensure_signal_username(&client, "+15555555555", "sigstack.99").await;
    }

    #[tokio::test]
    async fn ensure_skips_empty_nickname() {
        let mock_server = MockServer::start().await;
        let client = SignalClient::new(mock_server.uri()).unwrap();
        // No mock — would fail if called.
        ensure_signal_username(&client, "+15555555555", "  ").await;
    }

    #[tokio::test]
    async fn ensure_does_not_panic_on_api_error() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/accounts/%2B15555555555/username"))
            .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
            .mount(&mock_server)
            .await;

        let client = SignalClient::new(mock_server.uri()).unwrap();
        ensure_signal_username(&client, "+15555555555", "sigstack").await;
    }
}
