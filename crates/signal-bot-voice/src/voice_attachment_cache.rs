//! Recent voice note attachments for `!transcribe` quote-reply fallback.

use signal_client::Attachment;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

const DEFAULT_CAPACITY: usize = 1_000;

#[derive(Clone)]
struct CacheEntry {
    attachment: Attachment,
}

/// Maps `(chat_id, message_timestamp)` → voice attachment.
///
/// Signal quote metadata often omits downloadable attachment IDs for voice notes;
/// we populate this cache whenever a voice note arrives on a chat.
pub struct VoiceAttachmentCache {
    entries: RwLock<HashMap<(String, i64), CacheEntry>>,
    capacity: usize,
}

impl VoiceAttachmentCache {
    pub fn new(capacity: usize) -> Arc<Self> {
        Arc::new(Self {
            entries: RwLock::new(HashMap::new()),
            capacity: capacity.max(1),
        })
    }

    pub fn with_default_capacity() -> Arc<Self> {
        Self::new(DEFAULT_CAPACITY)
    }

    pub fn remember(&self, chat_id: &str, message_timestamp: i64, attachment: Attachment) {
        let key = (chat_id.to_string(), message_timestamp);
        let mut entries = self.entries.write().unwrap();
        if entries.len() >= self.capacity && !entries.contains_key(&key) {
            if let Some(oldest) = entries.keys().next().cloned() {
                entries.remove(&oldest);
            }
        }
        entries.insert(key, CacheEntry { attachment });
    }

    pub fn lookup(&self, chat_id: &str, message_timestamp: i64) -> Option<Attachment> {
        self.entries
            .read()
            .unwrap()
            .get(&(chat_id.to_string(), message_timestamp))
            .map(|entry| entry.attachment.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_audio(id: &str) -> Attachment {
        Attachment {
            content_type: "audio/ogg".into(),
            filename: None,
            id: id.into(),
            size: Some(1024),
            upload_timestamp: Some(1_719_000_000_000),
        }
    }

    #[test]
    fn remembers_and_lookup_by_chat_and_timestamp() {
        let cache = VoiceAttachmentCache::new(10);
        let audio = sample_audio("voice-1");
        cache.remember("group.abc", 1_719_000_000_000, audio.clone());

        let found = cache.lookup("group.abc", 1_719_000_000_000).unwrap();
        assert_eq!(found.id, "voice-1");
        assert!(cache.lookup("group.abc", 999).is_none());
        assert!(cache.lookup("other", 1_719_000_000_000).is_none());
    }
}
