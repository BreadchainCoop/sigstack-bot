//! Signal HTTP client.

use crate::error::SignalError;
use crate::types::*;
use reqwest::Client;
use std::time::Duration;
use tracing::{debug, instrument, warn};
use urlencoding::encode;

/// Signal CLI REST API client.
///
/// Supports multi-account operations - can send/receive for any registered account.
#[derive(Clone)]
pub struct SignalClient {
    client: Client,
    base_url: String,
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
        self.send(
            &original.receiving_account,
            original.reply_target(),
            message,
        )
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
            .or_else(|| {
                if original.text.is_empty() {
                    original
                        .primary_audio_attachment()
                        .map(|a| format!("[voice note: {}]", a.content_type))
                } else {
                    Some(truncate_snippet(&original.text, 120))
                }
            });

        self.send_v2(SendMessageV2Request {
            message: message.to_string(),
            number: original.receiving_account.clone(),
            recipients: vec![original.reply_target().to_string()],
            quote_timestamp: Some(original.message_timestamp),
            quote_author: Some(original.quote_author().to_string()),
            quote_message: snippet,
        })
        .await
    }
}

fn truncate_snippet(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_len).collect();
        format!("{truncated}…")
    }
}
