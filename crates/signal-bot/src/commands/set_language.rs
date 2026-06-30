//! `!set-en` / `!set-es` — per-group menu language for `!help` and `!privacy`.

use crate::commands::menu_locale::help_menu;
use crate::commands::CommandHandler;
use crate::error::AppResult;
use crate::group_preferences_store::GroupPreferencesStore;
use crate::menu_language::MenuLanguage;
use async_trait::async_trait;
use signal_client::BotMessage;
use std::sync::Arc;

const GROUP_ONLY_MSG: &str = "!set-es and !set-en are only available in group chats";

pub struct SetLanguageHandler {
    group_prefs: Arc<GroupPreferencesStore>,
}

impl SetLanguageHandler {
    pub fn new(group_prefs: Arc<GroupPreferencesStore>) -> Self {
        Self { group_prefs }
    }
}

#[async_trait]
impl CommandHandler for SetLanguageHandler {
    fn matches(&self, message: &BotMessage) -> bool {
        let text = message.text.trim();
        text == "!set-en" || text == "!set-es"
    }

    fn label(&self) -> &'static str {
        "set_language"
    }

    async fn execute(&self, message: &BotMessage) -> AppResult<String> {
        let group_id = match message.group_id.as_deref() {
            Some(id) => id,
            None => return Ok(GROUP_ONLY_MSG.into()),
        };

        let language = if message.text.trim() == "!set-es" {
            MenuLanguage::Es
        } else {
            MenuLanguage::En
        };

        self.group_prefs.set_menu_language(group_id, language);

        let confirmation = match language {
            MenuLanguage::En => "Menu language set to English for this group.",
            MenuLanguage::Es => "Idioma del menú configurado en español para este grupo.",
        };

        Ok(format!("{confirmation}\n\n{}", help_menu(language)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_handler() -> SetLanguageHandler {
        SetLanguageHandler::new(GroupPreferencesStore::new_in_memory(0))
    }

    #[test]
    fn matches_set_commands_only() {
        let handler = test_handler();
        let mut msg = BotMessage {
            source: "+1".into(),
            text: "!set-es".into(),
            timestamp: 0,
            message_timestamp: 0,
            is_group: true,
            group_id: Some("gid".into()),
            receiving_account: "+2".into(),
            attachments: vec![],
            quote: None,
        };
        assert!(handler.matches(&msg));
        msg.text = "!set-español".into();
        assert!(!handler.matches(&msg));
    }

    #[tokio::test]
    async fn sets_group_language() {
        let handler = test_handler();
        let msg = BotMessage {
            source: "+1".into(),
            text: "!set-es".into(),
            timestamp: 0,
            message_timestamp: 0,
            is_group: true,
            group_id: Some("gid".into()),
            receiving_account: "+2".into(),
            attachments: vec![],
            quote: None,
        };
        let response = handler.execute(&msg).await.unwrap();
        assert!(response.contains("español"));
        assert_eq!(
            handler.group_prefs.get_menu_language("gid"),
            MenuLanguage::Es
        );
    }
}
