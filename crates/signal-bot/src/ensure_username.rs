//! Ensure the bot has a Signal username for public discovery (no E.164 on the site).

use signal_client::{SignalClient, SignalError, UsernameInfo};
use thiserror::Error;
use tracing::{info, warn};

/// Prefix of Signal username share URLs (`https://signal.me/#eu/<token>`).
pub const SIGNAL_USERNAME_LINK_PREFIX: &str = "https://signal.me/#eu/";

/// Failure claiming / refreshing a Signal username.
#[derive(Debug, Error)]
pub enum ClaimUsernameError {
    #[error("Signal username nickname is empty")]
    EmptyNickname,
    #[error(transparent)]
    Signal(#[from] SignalError),
}

/// Extract the share token from a signal-cli `username_link`.
///
/// Accepts `https://signal.me/#eu/<token>` (case-insensitive host). Returns `None`
/// for empty, truncated, or unexpected links — never returns a full URL.
pub fn username_link_token(username_link: &str) -> Option<&str> {
    let trimmed = username_link.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with(SIGNAL_USERNAME_LINK_PREFIX) {
        return None;
    }
    let token = trimmed[SIGNAL_USERNAME_LINK_PREFIX.len()..].trim();
    if token.is_empty() || token.contains('#') || token.contains('/') {
        return None;
    }
    Some(token)
}

/// Normalize env nickname to Signal nickname only (strip accidental `.NN`).
fn normalize_nickname(nickname: &str) -> Result<&str, ClaimUsernameError> {
    let nickname = nickname.trim();
    if nickname.is_empty() {
        return Err(ClaimUsernameError::EmptyNickname);
    }
    let nickname = nickname
        .split_once('.')
        .map(|(nick, _)| nick)
        .unwrap_or(nickname)
        .trim();
    if nickname.is_empty() {
        return Err(ClaimUsernameError::EmptyNickname);
    }
    Ok(nickname)
}

/// Claim / refresh a Signal username via signal-cli.
///
/// Pass a nickname (e.g. `sigstack`); Signal assigns a discriminator and returns
/// `UsernameInfo` with the full username and shareable link.
pub async fn claim_signal_username(
    signal: &SignalClient,
    phone_number: &str,
    nickname: &str,
) -> Result<UsernameInfo, ClaimUsernameError> {
    let nickname = normalize_nickname(nickname)?;
    Ok(signal.set_username(phone_number, nickname).await?)
}

/// Claim / refresh a Signal username via signal-cli.
///
/// Non-fatal on failure so a username API hiccup does not block the bot.
/// On success, logs `username` + `username_token` for ops to set
/// `PUBLIC_SIGNAL_USERNAME_TOKEN` on the marketing site, and returns the
/// full claimed username (e.g. `sigstack.01`). Returns `None` on skip/failure.
pub async fn ensure_signal_username(
    signal: &SignalClient,
    phone_number: &str,
    nickname: &str,
) -> Option<String> {
    match claim_signal_username(signal, phone_number, nickname).await {
        Ok(info) => {
            let username = info.username.as_deref().filter(|u| !u.is_empty());
            let token = info
                .username_link
                .as_deref()
                .and_then(username_link_token)
                .unwrap_or("(no token returned)");
            info!(
                username = username.unwrap_or("(unknown)"),
                username_token = token,
                "Signal username ready — set site PUBLIC_SIGNAL_USERNAME_TOKEN to username_token after re-register"
            );
            username.map(str::to_string)
        }
        Err(ClaimUsernameError::EmptyNickname) => {
            if nickname.trim().is_empty() {
                info!("BOT__SIGNAL_USERNAME empty — skipping Signal username ensure");
            } else {
                warn!("BOT__SIGNAL_USERNAME has no nickname — skipping Signal username ensure");
            }
            None
        }
        Err(err) => {
            warn!(
                error = %err,
                nickname,
                "Failed to ensure Signal username (non-fatal); set manually via signal-cli if needed"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn username_link_token_extracts_payload() {
        assert_eq!(
            username_link_token("https://signal.me/#eu/abcTOKEN"),
            Some("abcTOKEN")
        );
        assert_eq!(
            username_link_token("  HTTPS://SIGNAL.ME/#eu/xyz  "),
            Some("xyz")
        );
    }

    #[test]
    fn username_link_token_rejects_bad_links() {
        assert_eq!(username_link_token(""), None);
        assert_eq!(username_link_token("   "), None);
        assert_eq!(username_link_token("https://signal.me/"), None);
        assert_eq!(username_link_token("https://signal.me/#eu/"), None);
        assert_eq!(username_link_token("https://example.com/#eu/abc"), None);
        assert_eq!(username_link_token("abcTOKEN"), None);
        assert_eq!(username_link_token("https://signal.me/#eu/a/b"), None);
    }

    #[tokio::test]
    async fn claim_returns_username_info() {
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
        let info = claim_signal_username(&client, "+15555555555", "sigstack.99")
            .await
            .unwrap();
        assert_eq!(info.username.as_deref(), Some("sigstack.01"));
        assert_eq!(
            info.username_link.as_deref(),
            Some("https://signal.me/#eu/x")
        );
        assert_eq!(
            username_link_token(info.username_link.as_deref().unwrap()),
            Some("x")
        );
    }

    #[tokio::test]
    async fn claim_rejects_empty_nickname() {
        let mock_server = MockServer::start().await;
        let client = SignalClient::new(mock_server.uri()).unwrap();
        let err = claim_signal_username(&client, "+15555555555", "  ")
            .await
            .unwrap_err();
        assert!(matches!(err, ClaimUsernameError::EmptyNickname));
    }

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
        let got = ensure_signal_username(&client, "+15555555555", "sigstack.99").await;
        assert_eq!(got.as_deref(), Some("sigstack.01"));
    }

    #[tokio::test]
    async fn ensure_skips_empty_nickname() {
        let mock_server = MockServer::start().await;
        let client = SignalClient::new(mock_server.uri()).unwrap();
        // No mock — would fail if called.
        assert!(ensure_signal_username(&client, "+15555555555", "  ")
            .await
            .is_none());
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
        assert!(ensure_signal_username(&client, "+15555555555", "sigstack")
            .await
            .is_none());
    }
}
