//! Help command - displays feature menu.

use crate::commands::menu_locale::{help_menu, menu_language_for_message};
use crate::commands::CommandHandler;
use crate::config::BotRole;
use crate::error::AppResult;
use crate::group_preferences_store::GroupPreferencesStore;
use async_trait::async_trait;
use signal_client::BotMessage;
use std::sync::Arc;

pub struct HelpHandler {
    group_prefs: Arc<GroupPreferencesStore>,
    role: BotRole,
}

impl HelpHandler {
    pub fn new(group_prefs: Arc<GroupPreferencesStore>, role: BotRole) -> Self {
        Self { group_prefs, role }
    }
}

#[async_trait]
impl CommandHandler for HelpHandler {
    fn trigger(&self) -> Option<&str> {
        Some("!help")
    }

    fn label(&self) -> &'static str {
        "help"
    }

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        let language = menu_language_for_message(message, &self.group_prefs);
        Ok(help_menu(language, self.role).into())
    }
}
