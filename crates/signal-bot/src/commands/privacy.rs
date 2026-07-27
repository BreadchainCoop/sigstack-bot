//! `!privacy` — privacy, security, and TEE commands menu.

use crate::commands::menu_locale::{menu_language_for_message, privacy_menu};
use crate::commands::CommandHandler;
use crate::config::BotRole;
use crate::error::AppResult;
use crate::group_preferences_store::GroupPreferencesStore;
use async_trait::async_trait;
use signal_client::BotMessage;
use std::sync::Arc;

pub struct PrivacyHandler {
    group_prefs: Arc<GroupPreferencesStore>,
    role: BotRole,
}

impl PrivacyHandler {
    pub fn new(group_prefs: Arc<GroupPreferencesStore>, role: BotRole) -> Self {
        Self { group_prefs, role }
    }
}

#[async_trait]
impl CommandHandler for PrivacyHandler {
    fn trigger(&self) -> Option<&str> {
        Some("!privacy")
    }

    fn label(&self) -> &'static str {
        "privacy"
    }

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        let language = menu_language_for_message(message, &self.group_prefs);
        Ok(privacy_menu(language, self.role).into())
    }
}
