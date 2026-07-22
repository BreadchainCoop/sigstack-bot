//! Track the bot's Signal phone(s) and UUID so we never relay our own messages.

use signal_client::BotMessage;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

/// In-memory bot identity (phones from `receiving_account`, UUID learned if seen).
#[derive(Debug, Default)]
pub struct BotIdentity {
    phones: RwLock<HashSet<String>>,
    uuids: RwLock<HashSet<String>>,
}

impl BotIdentity {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn remember_phone(&self, phone: &str) {
        if phone.is_empty() {
            return;
        }
        self.phones.write().unwrap().insert(phone.to_string());
    }

    pub fn learn_uuid(&self, uuid: &str) {
        if uuid.is_empty() || uuid.starts_with('+') {
            return;
        }
        self.uuids.write().unwrap().insert(uuid.to_string());
    }

    /// Call on every inbound message: remember account phone; learn UUID if self-sourced.
    pub fn note_inbound(&self, message: &BotMessage) {
        self.remember_phone(&message.receiving_account);

        let phones = self.phones.read().unwrap();
        let is_self = phones.contains(&message.source)
            || message
                .source_number
                .as_ref()
                .is_some_and(|n| phones.contains(n));
        drop(phones);

        if is_self && !message.source.starts_with('+') {
            self.learn_uuid(&message.source);
        }
    }

    pub fn is_bot_message(&self, message: &BotMessage) -> bool {
        let phones = self.phones.read().unwrap();
        let uuids = self.uuids.read().unwrap();

        if phones.contains(&message.source) || uuids.contains(&message.source) {
            return true;
        }
        if let Some(n) = &message.source_number {
            if phones.contains(n) {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(source: &str, source_number: Option<&str>, account: &str) -> BotMessage {
        BotMessage {
            source: source.into(),
            source_number: source_number.map(str::to_string),
            source_name: None,
            text: "hi".into(),
            timestamp: 1,
            message_timestamp: 1,
            is_group: true,
            group_id: Some("g".into()),
            group_name: None,
            receiving_account: account.into(),
            attachments: vec![],
            quote: None,
        }
    }

    #[test]
    fn skips_bot_phone_as_source() {
        let id = BotIdentity::new();
        let m = msg("+15550001111", Some("+15550001111"), "+15550001111");
        id.note_inbound(&m);
        assert!(id.is_bot_message(&m));
    }

    #[test]
    fn skips_bot_uuid_after_learn() {
        let id = BotIdentity::new();
        id.remember_phone("+15550001111");
        let self_msg = msg(
            "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            Some("+15550001111"),
            "+15550001111",
        );
        id.note_inbound(&self_msg);
        assert!(id.is_bot_message(&self_msg));

        let again = msg(
            "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            None,
            "+15550001111",
        );
        assert!(id.is_bot_message(&again));
    }

    #[test]
    fn human_not_bot() {
        let id = BotIdentity::new();
        id.remember_phone("+15550001111");
        let m = msg("+15550002222", Some("+15550002222"), "+15550001111");
        assert!(!id.is_bot_message(&m));
    }
}
