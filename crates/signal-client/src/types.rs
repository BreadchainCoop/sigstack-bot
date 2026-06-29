//! Signal API types.

use serde::{Deserialize, Serialize};

/// Incoming Signal message.
#[derive(Debug, Clone, Deserialize)]
pub struct IncomingMessage {
    pub envelope: Envelope,
    pub account: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Envelope {
    pub source: String,
    #[serde(rename = "sourceNumber")]
    pub source_number: Option<String>,
    #[serde(rename = "sourceName")]
    pub source_name: Option<String>,
    pub timestamp: i64,
    #[serde(rename = "dataMessage")]
    pub data_message: Option<DataMessage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DataMessage {
    pub message: Option<String>,
    pub timestamp: i64,
    #[serde(rename = "groupInfo")]
    pub group_info: Option<GroupInfo>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    pub quote: Option<Quote>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GroupInfo {
    /// Internal group ID on incoming messages (`internal_id` from list groups).
    #[serde(rename = "groupId")]
    pub group_id: String,
    #[serde(rename = "groupName")]
    pub group_name: Option<String>,
}

/// Group from `GET /v1/groups/{number}` — use `id` (not `internal_id`) for `/v2/send`.
#[derive(Debug, Clone, Deserialize)]
pub struct Group {
    pub name: String,
    pub id: String,
    #[serde(rename = "internal_id")]
    pub internal_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Attachment {
    #[serde(rename = "contentType")]
    pub content_type: String,
    pub filename: Option<String>,
    pub id: String,
    pub size: Option<i64>,
    #[serde(rename = "uploadTimestamp")]
    pub upload_timestamp: Option<i64>,
}

impl Attachment {
    /// Whether this attachment is an audio/voice note.
    pub fn is_audio(&self) -> bool {
        self.content_type.starts_with("audio/")
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Quote {
    pub id: i64,
    pub author: Option<String>,
    #[serde(rename = "authorNumber")]
    pub author_number: Option<String>,
    #[serde(rename = "authorUuid")]
    pub author_uuid: Option<String>,
    pub text: Option<String>,
    #[serde(default)]
    pub attachments: Vec<QuotedAttachment>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuotedAttachment {
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
    pub filename: Option<String>,
    pub thumbnail: Option<Attachment>,
}

/// Quote metadata on an incoming message (user replied to another message).
#[derive(Debug, Clone)]
pub struct QuotedMessage {
    pub id: i64,
    pub author_number: Option<String>,
    pub text: Option<String>,
}

/// Outgoing message request (legacy shape; prefer [`SendMessageV2Request`]).
#[derive(Debug, Clone, Serialize)]
pub struct SendMessageRequest {
    pub message: String,
    pub number: Option<String>,
    pub recipients: Option<Vec<String>>,
}

/// Outgoing message request for `/v2/send` (text + optional quote-reply).
#[derive(Debug, Clone, Serialize)]
pub struct SendMessageV2Request {
    pub message: String,
    pub number: String,
    pub recipients: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote_timestamp: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote_author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote_message: Option<String>,
}

/// Send message response.
#[derive(Debug, Clone, Deserialize)]
pub struct SendMessageResponse {
    pub timestamp: Option<i64>,
}

/// Account information.
#[derive(Debug, Clone, Deserialize)]
pub struct Account {
    pub number: String,
    pub uuid: Option<String>,
    pub registered: bool,
}

/// Parsed message for bot processing.
#[derive(Debug, Clone)]
pub struct BotMessage {
    /// The phone number that sent the message.
    pub source: String,
    /// The message text (empty for voice-only messages).
    pub text: String,
    /// Envelope timestamp (milliseconds).
    pub timestamp: i64,
    /// `dataMessage.timestamp` — use for outbound `quote_timestamp`.
    pub message_timestamp: i64,
    /// Whether this is a group message.
    pub is_group: bool,
    /// Group ID if this is a group message.
    pub group_id: Option<String>,
    /// The bot's phone number that received this message.
    pub receiving_account: String,
    /// Attachments on this message (voice notes, etc.).
    pub attachments: Vec<Attachment>,
    /// Quote/reply metadata if the sender quoted another message.
    pub quote: Option<QuotedMessage>,
}

impl BotMessage {
    /// Extract bot message from incoming envelope.
    ///
    /// Returns `Some` for text messages and voice notes (audio attachments).
    pub fn from_incoming(msg: &IncomingMessage) -> Option<Self> {
        let data = msg.envelope.data_message.as_ref()?;
        let has_text = data
            .message
            .as_ref()
            .is_some_and(|text| !text.is_empty());
        let has_audio = data.attachments.iter().any(Attachment::is_audio);

        if !has_text && !has_audio {
            return None;
        }

        let quote = data.quote.as_ref().map(|q| QuotedMessage {
            id: q.id,
            author_number: q.author_number.clone().or_else(|| q.author.clone()),
            text: q.text.clone(),
        });

        Some(Self {
            source: msg.envelope.source.clone(),
            text: data.message.clone().unwrap_or_default(),
            timestamp: msg.envelope.timestamp,
            message_timestamp: data.timestamp,
            is_group: data.group_info.is_some(),
            group_id: data.group_info.as_ref().map(|g| g.group_id.clone()),
            receiving_account: msg.account.clone(),
            attachments: data.attachments.clone(),
            quote,
        })
    }

    /// Whether this message is a voice note (has at least one audio attachment).
    pub fn is_voice_note(&self) -> bool {
        self.attachments.iter().any(Attachment::is_audio)
    }

    /// Audio attachments only (voice notes).
    pub fn audio_attachments(&self) -> impl Iterator<Item = &Attachment> {
        self.attachments.iter().filter(|a| a.is_audio())
    }

    /// Primary audio attachment, if any (first audio attachment).
    pub fn primary_audio_attachment(&self) -> Option<&Attachment> {
        self.audio_attachments().next()
    }

    /// Raw reply target from the envelope (DM source or group `internal_id`).
    ///
    /// For group sends, resolve via [`SignalClient::resolve_send_recipient`] — `/v2/send`
    /// requires the `group.*` id from list groups, not `groupInfo.groupId` on receive.
    pub fn reply_target(&self) -> &str {
        self.group_id.as_deref().unwrap_or(&self.source)
    }

    /// Author identifier for quoting this message (`source` phone number).
    pub fn quote_author(&self) -> &str {
        &self.source
    }
}
