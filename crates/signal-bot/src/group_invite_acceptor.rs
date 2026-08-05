//! Auto-accept pending Signal group invites.
//!
//! - **Translation:** accept any pending invite/request (MVP).
//! - **Transcription:** accept only when the translation peer is already a member/admin.

use crate::config::BotRole;
use signal_client::{Group, SignalClient};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

/// How often to scan `GET /v1/groups` for pending invites.
pub const DEFAULT_INVITE_POLL_INTERVAL: Duration = Duration::from_secs(5);

/// Policy for which pending invites to accept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvitePolicy {
    /// Accept every group where this account is pending (translation hub).
    AcceptAll,
    /// Accept only when `peer` is already a member or admin (transcription worker).
    AcceptIfPeerPresent { peer: String },
}

impl InvitePolicy {
    /// Build policy from bot role and optional peer phone.
    ///
    /// Transcription without `PEER_PHONE` refuses all invites.
    pub fn for_role(role: BotRole, peer_phone: Option<&str>) -> Option<Self> {
        match role {
            BotRole::Translation => Some(Self::AcceptAll),
            BotRole::Transcription => {
                let peer = peer_phone?.trim();
                if peer.is_empty() {
                    return None;
                }
                Some(Self::AcceptIfPeerPresent {
                    peer: peer.to_string(),
                })
            }
        }
    }
}

/// Whether this account should `POST .../join` for `group`.
pub fn should_join(group: &Group, self_identity: &str, policy: &InvitePolicy) -> bool {
    if !group.is_pending_for(self_identity) {
        return false;
    }
    match policy {
        InvitePolicy::AcceptAll => true,
        InvitePolicy::AcceptIfPeerPresent { peer } => group.has_member_or_admin(peer),
    }
}

/// One scan: list groups and join those that match policy.
pub async fn accept_pending_invites(
    signal: &SignalClient,
    phone_number: &str,
    policy: &InvitePolicy,
) -> usize {
    let groups = match signal.list_groups(phone_number).await {
        Ok(g) => g,
        Err(e) => {
            warn!(error = %e, "Failed to list groups for invite accept");
            return 0;
        }
    };

    let mut joined = 0;
    for group in groups {
        if !should_join(&group, phone_number, policy) {
            continue;
        }
        match signal.join_group(phone_number, &group.id).await {
            Ok(()) => {
                info!(
                    group_id = %group.id,
                    group_name = %group.name,
                    "Accepted pending group invite"
                );
                joined += 1;
            }
            Err(e) => {
                warn!(
                    error = %e,
                    group_id = %group.id,
                    group_name = %group.name,
                    "Failed to join group from pending invite"
                );
            }
        }
    }
    joined
}

/// Resolve which Signal account to poll (configured phone, else first registered).
pub async fn resolve_account_phone(
    signal: &SignalClient,
    configured: Option<&str>,
) -> Option<String> {
    if let Some(phone) = configured.map(str::trim).filter(|p| !p.is_empty()) {
        return Some(phone.to_string());
    }
    match signal.list_accounts().await {
        Ok(accounts) => accounts.into_iter().next(),
        Err(e) => {
            warn!(error = %e, "Failed to list accounts for invite accept");
            None
        }
    }
}

/// Background loop: periodically accept pending invites per policy.
pub async fn run_invite_acceptor(
    signal: Arc<SignalClient>,
    phone_number: Option<String>,
    policy: InvitePolicy,
    poll_interval: Duration,
) {
    info!(
        ?policy,
        interval_secs = poll_interval.as_secs(),
        "Group invite acceptor started"
    );
    loop {
        if let Some(account) = resolve_account_phone(&signal, phone_number.as_deref()).await {
            let n = accept_pending_invites(&signal, &account, &policy).await;
            if n == 0 {
                debug!("No pending group invites to accept");
            }
        } else {
            warn!("No Signal account available for invite accept; retrying");
        }
        tokio::time::sleep(poll_interval).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn group_json(
        id: &str,
        members: &[&str],
        pending: &[&str],
        admins: &[&str],
    ) -> serde_json::Value {
        json!({
            "name": "G",
            "id": id,
            "internal_id": format!("{id}-internal"),
            "members": members,
            "pending_invites": pending,
            "pending_requests": [],
            "admins": admins
        })
    }

    fn sample_group(members: &[&str], pending: &[&str], admins: &[&str]) -> Group {
        serde_json::from_value(group_json("group.abc==", members, pending, admins)).unwrap()
    }

    #[test]
    fn policy_for_role() {
        assert_eq!(
            InvitePolicy::for_role(BotRole::Translation, None),
            Some(InvitePolicy::AcceptAll)
        );
        assert_eq!(
            InvitePolicy::for_role(BotRole::Transcription, Some("+15550009999")),
            Some(InvitePolicy::AcceptIfPeerPresent {
                peer: "+15550009999".into()
            })
        );
        assert!(InvitePolicy::for_role(BotRole::Transcription, None).is_none());
        assert!(InvitePolicy::for_role(BotRole::Transcription, Some("  ")).is_none());
    }

    #[test]
    fn translation_accepts_any_pending() {
        let policy = InvitePolicy::AcceptAll;
        let pending = sample_group(&["+15550001111"], &["+15550002222"], &["+15550001111"]);
        assert!(should_join(&pending, "+15550002222", &policy));
        assert!(!should_join(&pending, "+15550001111", &policy));
    }

    #[test]
    fn transcription_requires_peer_member() {
        let policy = InvitePolicy::AcceptIfPeerPresent {
            peer: "+15550003333".into(),
        };
        let with_peer = sample_group(
            &["+15550003333", "+15550001111"],
            &["+15550002222"],
            &["+15550003333"],
        );
        assert!(should_join(&with_peer, "+15550002222", &policy));

        let without_peer = sample_group(&["+15550001111"], &["+15550002222"], &["+15550001111"]);
        assert!(!should_join(&without_peer, "+15550002222", &policy));
    }

    #[tokio::test]
    async fn accept_pending_joins_matching_groups() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/groups/%2B15550002222"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                group_json("group.skip==", &["+15550001111"], &[], &["+15550001111"]),
                group_json(
                    "group.join==",
                    &["+15550001111"],
                    &["+15550002222"],
                    &["+15550001111"]
                ),
            ])))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/v1/groups/%2B15550002222/group.join%3D%3D/join"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&server)
            .await;

        let signal = SignalClient::new(server.uri()).unwrap();
        let n = accept_pending_invites(&signal, "+15550002222", &InvitePolicy::AcceptAll).await;
        assert_eq!(n, 1);
    }

    #[tokio::test]
    async fn transcription_skips_when_peer_absent() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/groups/%2B15550002222"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([group_json(
                "group.nopeer==",
                &["+15550001111"],
                &["+15550002222"],
                &["+15550001111"]
            )])))
            .mount(&server)
            .await;

        let signal = SignalClient::new(server.uri()).unwrap();
        let policy = InvitePolicy::AcceptIfPeerPresent {
            peer: "+15550003333".into(),
        };
        let n = accept_pending_invites(&signal, "+15550002222", &policy).await;
        assert_eq!(n, 0);
    }
}
