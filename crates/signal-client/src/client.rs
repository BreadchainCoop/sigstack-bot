//! Signal HTTP client.

use crate::error::SignalError;
use crate::types::*;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, instrument, warn};
use urlencoding::encode;

type GroupCache = Arc<RwLock<HashMap<(String, String), String>>>;

/// Signal CLI REST API client.
///
/// Supports multi-account operations - can send/receive for any registered account.
#[derive(Clone)]
pub struct SignalClient {
    client: Client,
    base_url: String,
    group_cache: GroupCache,
}

impl SignalClient {
    /// Create a new Signal client.
    pub fn new(base_url: impl Into<String>) -> Result<Self, SignalError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            base_url: base_url.into(),
            group_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// List all registered accounts.
    #[instrument(skip(self))]
    pub async fn list_accounts(&self) -> Result<Vec<String>, SignalError> {
        let response = self
            .client
            .get(format!("{}/v1/accounts", self.base_url))
            .send()
            .await?;

        if !response.status().is_success() {
            let msg = response.text().await.unwrap_or_default();
            return Err(SignalError::Api(msg));
        }

        let accounts: Vec<String> = response.json().await?;
        debug!("Found {} registered accounts", accounts.len());
        Ok(accounts)
    }

    /// List Signal groups for an account.
    #[instrument(skip(self))]
    pub async fn list_groups(&self, phone_number: &str) -> Result<Vec<Group>, SignalError> {
        let encoded_number = encode(phone_number);
        let response = self
            .client
            .get(format!("{}/v1/groups/{}", self.base_url, encoded_number))
            .send()
            .await?;

        if !response.status().is_success() {
            let msg = response.text().await.unwrap_or_default();
            return Err(SignalError::Api(msg));
        }

        let groups: Vec<Group> = response.json().await?;
        debug!("Found {} groups for {}", groups.len(), phone_number);
        Ok(groups)
    }

    /// Resolve the recipient id for `/v2/send` (DM source or `group.*` id).
    #[instrument(skip(self))]
    pub async fn resolve_send_recipient(
        &self,
        message: &BotMessage,
    ) -> Result<String, SignalError> {
        let Some(group_id) = message.group_id.as_deref() else {
            return Ok(message.source.clone());
        };

        if group_id.starts_with("group.") {
            return Ok(group_id.to_string());
        }

        let cache_key = (message.receiving_account.clone(), group_id.to_string());
        if let Some(send_id) = self.group_cache.read().await.get(&cache_key) {
            return Ok(send_id.clone());
        }

        let groups = self.list_groups(&message.receiving_account).await?;
        let send_id = match resolve_group_send_id(group_id, &groups) {
            Some(id) => id,
            None => {
                return Err(SignalError::Api(format!(
                    "Unknown group id {group_id} for account {}",
                    message.receiving_account
                )));
            }
        };

        self.group_cache
            .write()
            .await
            .insert(cache_key, send_id.clone());

        debug!(
            "Resolved group internal_id {} -> {}",
            group_id, send_id
        );
        Ok(send_id)
    }

    /// Check if the Signal API is healthy.
    pub async fn health_check(&self) -> bool {
        self.client
            .get(format!("{}/v1/health", self.base_url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Get account information for a specific phone number.
    #[instrument(skip(self))]
    pub async fn get_account(&self, phone_number: &str) -> Result<Account, SignalError> {
        let encoded_number = encode(phone_number);
        let response = self
            .client
            .get(format!("{}/v1/accounts/{}", self.base_url, encoded_number))
            .send()
            .await?;

        if !response.status().is_success() {
            let msg = response.text().await.unwrap_or_default();
            return Err(SignalError::Api(msg));
        }

        Ok(response.json().await?)
    }

    /// Receive pending messages for a specific phone number.
    #[instrument(skip(self))]
    pub async fn receive(&self, phone_number: &str) -> Result<Vec<IncomingMessage>, SignalError> {
        let encoded_number = encode(phone_number);
        let response = self
            .client
            .get(format!(
                "{}/v1/receive/{}",
                self.base_url, encoded_number
            ))
            .send()
            .await?;

        if !response.status().is_success() {
            let msg = response.text().await.unwrap_or_default();
            return Err(SignalError::Api(msg));
        }

        let messages: Vec<IncomingMessage> = response.json().await?;
        debug!("Received {} messages for {}", messages.len(), phone_number);
        Ok(messages)
    }

    /// Download attachment bytes by ID (auto-downloaded during receive).
    #[instrument(skip(self))]
    pub async fn download_attachment(&self, attachment_id: &str) -> Result<Vec<u8>, SignalError> {
        let encoded_id = encode(attachment_id);
        let response = self
            .client
            .get(format!(
                "{}/v1/attachments/{}",
                self.base_url, encoded_id
            ))
            .send()
            .await?;

        if !response.status().is_success() {
            let msg = response.text().await.unwrap_or_default();
            return Err(SignalError::AttachmentDownloadFailed(msg));
        }

        let bytes = response.bytes().await?.to_vec();
        debug!(
            "Downloaded attachment {} ({} bytes)",
            attachment_id,
            bytes.len()
        );
        Ok(bytes)
    }

    /// Send a message from a specific account to a recipient.
    #[instrument(skip(self, message))]
    pub async fn send(
        &self,
        from_number: &str,
        recipient: &str,
        message: &str,
    ) -> Result<(), SignalError> {
        self.send_v2(SendMessageV2Request {
            message: message.to_string(),
            number: from_number.to_string(),
            recipients: vec![recipient.to_string()],
            quote_timestamp: None,
            quote_author: None,
            quote_message: None,
        })
        .await
    }

    /// Send a message via `/v2/send` with optional quote-reply fields.
    #[instrument(skip(self, request))]
    pub async fn send_v2(&self, request: SendMessageV2Request) -> Result<(), SignalError> {
        let response = self
            .client
            .post(format!("{}/v2/send", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let msg = response.text().await.unwrap_or_default();
            warn!("Send failed: {}", msg);
            return Err(SignalError::SendFailed(msg));
        }

        debug!(
            "Sent message from {} to {:?}",
            request.number, request.recipients
        );
        Ok(())
    }

    /// Reply to a message (handles both direct and group messages).
    /// Uses the receiving account to send the reply.
    pub async fn reply(&self, original: &BotMessage, message: &str) -> Result<(), SignalError> {
        let recipient = self.resolve_send_recipient(original).await?;
        self.send(&original.receiving_account, &recipient, message)
            .await
    }

    /// Quote-reply to a message (threads bot output to the original).
    #[instrument(skip(self, message))]
    pub async fn reply_quoted(
        &self,
        original: &BotMessage,
        message: &str,
        quote_snippet: Option<&str>,
    ) -> Result<(), SignalError> {
        let snippet = quote_snippet
            .map(str::to_string)
            .or_else(|| quote_snippet_for_message(original));

        self.reply_quoted_target(
            original,
            original.message_timestamp,
            original.quote_author(),
            snippet.as_deref(),
            message,
        )
        .await
    }

    /// Quote-reply to a specific message (e.g. the message quoted by the user's command).
    #[instrument(skip(self, message))]
    pub async fn reply_quoted_target(
        &self,
        context: &BotMessage,
        quote_timestamp: i64,
        quote_author: &str,
        quote_snippet: Option<&str>,
        message: &str,
    ) -> Result<(), SignalError> {
        let recipient = self.resolve_send_recipient(context).await?;

        self.send_v2(SendMessageV2Request {
            message: message.to_string(),
            number: context.receiving_account.clone(),
            recipients: vec![recipient],
            quote_timestamp: Some(quote_timestamp),
            quote_author: Some(quote_author.to_string()),
            quote_message: quote_snippet.map(str::to_string),
        })
        .await
    }
}

/// Map incoming `groupInfo.groupId` (`internal_id`) to list-groups `id` for send.
pub fn resolve_group_send_id(incoming_group_id: &str, groups: &[Group]) -> Option<String> {
    if incoming_group_id.starts_with("group.") {
        return Some(incoming_group_id.to_string());
    }

    groups
        .iter()
        .find(|g| g.internal_id == incoming_group_id)
        .map(|g| g.id.clone())
}

fn truncate_snippet(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_len).collect();
        format!("{truncated}…")
    }
}

fn quote_snippet_for_message(original: &BotMessage) -> Option<String> {
    if original.text.is_empty() {
        original
            .primary_audio_attachment()
            .map(|a| format!("[voice note: {}]", a.content_type))
    } else {
        Some(truncate_snippet(&original.text, 120))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_group_send_id_maps_internal_id() {
        let groups = vec![Group {
            name: "test".into(),
            id: "group.TUIzYitaQy85SmtteUpTMEo2ZE9wZ3lib0tOWVZrcDEzNFA3bDU0N1BrOD0=".into(),
            internal_id: "MB3b+ZC/9JkmyJS0J6dOpgyboKNYVkp134P7l547Pk8=".into(),
        }];

        assert_eq!(
            resolve_group_send_id("MB3b+ZC/9JkmyJS0J6dOpgyboKNYVkp134P7l547Pk8=", &groups),
            Some("group.TUIzYitaQy85SmtteUpTMEo2ZE9wZ3lib0tOWVZrcDEzNFA3bDU0N1BrOD0=".into())
        );
    }

    #[test]
    fn resolve_group_send_id_passes_through_send_id() {
        let groups = vec![];
        assert_eq!(
            resolve_group_send_id("group.abc123=", &groups),
            Some("group.abc123=".into())
        );
    }
}
