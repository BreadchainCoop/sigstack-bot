//! Aggregate command alias tables and collision checks.

use crate::commands::menu_locale::{
    COMMANDS_COMMANDS, HELP_COMMANDS, HELP_IN_CHAT_COMMANDS, HELP_THREADS_COMMANDS,
    HELP_TRANSCRIPTION_COMMANDS, INFO_COMMANDS, IN_CHAT_MENU_COMMANDS, PRIVACY_COMMANDS,
    TRANSCRIPTION_MENU_COMMANDS, TRANSLATION_IN_CHAT_MENU_COMMANDS, TRANSLATION_REDIRECT_COMMANDS,
    TRANSLATION_THREADS_MENU_COMMANDS,
};
use crate::commands::translate_all::{
    ALL_OFF_COMMANDS, ALL_ON_PREFIXES, ENABLE_THREADS, ME_OFF_COMMANDS, ME_ON_PREFIXES,
};
use crate::commands::translate_langs::LIST_LANGS_COMMANDS;
use crate::commands::translate_me::{ENABLE_IN_CHAT_CMDS, LEAVE_CMDS, THREAD_ON_PREFIXES};
use signal_bot_core::normalize_exact;
use signal_bot_voice::{TRANSCRIBE_COMMANDS, TRANSCRIBE_OFF_COMMANDS, TRANSCRIBE_ON_COMMANDS};
use std::collections::HashMap;

const RENAME_PREFIXES: &[&str] = &["!rename"];
const VERIFY_PREFIXES: &[&str] = &["!verify"];

const RESERVED_STEMS: &[&str] = &[
    "!transcript",
    "!translate-me",
    "!translate-all",
    "!transcription-on",
    "!transcription-off",
];

fn all_families() -> &'static [(&'static str, &'static [&'static str])] {
    &[
        ("help", HELP_COMMANDS),
        ("info", INFO_COMMANDS),
        ("privacy", PRIVACY_COMMANDS),
        ("commands", COMMANDS_COMMANDS),
        ("translation_redirect", TRANSLATION_REDIRECT_COMMANDS),
        (
            "translation_threads_menu",
            TRANSLATION_THREADS_MENU_COMMANDS,
        ),
        (
            "translation_in_chat_menu",
            TRANSLATION_IN_CHAT_MENU_COMMANDS,
        ),
        ("in_chat_menu", IN_CHAT_MENU_COMMANDS),
        ("transcription_menu", TRANSCRIPTION_MENU_COMMANDS),
        ("help_threads", HELP_THREADS_COMMANDS),
        ("help_in_chat", HELP_IN_CHAT_COMMANDS),
        ("help_transcription", HELP_TRANSCRIPTION_COMMANDS),
        ("translate_all_on", ALL_ON_PREFIXES),
        ("translate_all_off", ALL_OFF_COMMANDS),
        ("translate_me_on", ME_ON_PREFIXES),
        ("translate_me_off", ME_OFF_COMMANDS),
        ("enable_threads", ENABLE_THREADS),
        ("thread_on", THREAD_ON_PREFIXES),
        ("enable_in_chat", ENABLE_IN_CHAT_CMDS),
        ("leave", LEAVE_CMDS),
        ("list_langs", LIST_LANGS_COMMANDS),
        ("transcribe_on", TRANSCRIBE_ON_COMMANDS),
        ("transcribe_off", TRANSCRIBE_OFF_COMMANDS),
        ("transcribe", TRANSCRIBE_COMMANDS),
        ("rename", RENAME_PREFIXES),
        ("verify", VERIFY_PREFIXES),
    ]
}

fn register_aliases<'a>(
    map: &mut HashMap<String, &'a str>,
    id: &'a str,
    aliases: &[&str],
) -> Result<(), String> {
    for alias in aliases {
        let key = normalize_exact(alias);
        if let Some(prev) = map.get(&key) {
            if *prev != id {
                return Err(format!(
                    "alias `{alias}` (key `{key}`) maps to both `{prev}` and `{id}`"
                ));
            }
        } else {
            map.insert(key, id);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::translate::TranslateHandler;
    use crate::commands::CommandHandler;
    use near_ai_client::NearAiClient;
    use signal_client::{BotMessage, SignalClient};
    use std::sync::Arc;
    use std::time::Duration;

    fn build_map() -> HashMap<String, &'static str> {
        let mut map = HashMap::new();
        for (id, aliases) in all_families() {
            register_aliases(&mut map, id, aliases).expect(id);
        }
        map
    }

    #[test]
    fn alias_tables_are_unique_across_families() {
        let map = build_map();
        assert!(map.contains_key("!help-thread"));
        assert!(map.contains_key("!help thread"));
        assert!(map.contains_key("!enable-thread"));
        assert!(map.contains_key("!translate-me-threads"));
        assert!(map.contains_key("!transcript-on"));
        assert_eq!(map.get("!help-thread"), Some(&"help_threads"));
        assert_eq!(map.get("!help"), Some(&"help"));
        assert_ne!(map.get("!help-thread"), map.get("!help"));
    }

    #[test]
    fn uniqueness_detects_cross_family_duplicate() {
        let mut map = HashMap::new();
        register_aliases(&mut map, "help", &["!help"]).unwrap();
        let err = register_aliases(&mut map, "help_threads", &["!help"]).unwrap_err();
        assert!(err.contains("`!help`"));
        assert!(err.contains("help_threads"));
    }

    #[test]
    fn reserved_stems_are_absent() {
        let map = build_map();
        for stem in RESERVED_STEMS {
            assert!(
                !map.contains_key(*stem),
                "reserved stem `{stem}` must not be an alias"
            );
        }
        assert!(!map.contains_key("!transcript"));
    }

    fn quote_handler() -> TranslateHandler {
        TranslateHandler::new(
            Arc::new(
                NearAiClient::new("key", "http://localhost", "model", Duration::from_secs(5))
                    .unwrap(),
            ),
            Arc::new(SignalClient::new("http://localhost").unwrap()),
            "📝 Transcript:",
        )
    }

    fn msg(text: &str) -> BotMessage {
        BotMessage {
            source: "+1".into(),
            source_number: None,
            source_name: None,
            text: text.into(),
            timestamp: 0,
            message_timestamp: 0,
            is_group: true,
            group_id: Some("g".into()),
            group_name: None,
            receiving_account: "+2".into(),
            attachments: vec![],
            quote: None,
        }
    }

    #[test]
    fn quote_translate_excludes_every_non_quote_alias() {
        let handler = quote_handler();
        for (id, aliases) in all_families() {
            if *id == "verify" || *id == "rename" {
                continue;
            }
            for alias in *aliases {
                assert!(
                    !handler.matches(&msg(alias)),
                    "{alias} ({id}) must not match quote-translate"
                );
            }
        }
        assert!(handler.matches(&msg("!translate es")));
    }
}
