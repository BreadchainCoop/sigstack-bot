//! English `!help`, product, and `!privacy` menu text.
//!
//! Menus are English-only for now; multi-language UI is deferred.
//! Command-list layout follows the Signal mobile menu standard:
//! [`docs/solutions/signal-mobile-menus.md`](../../../../docs/solutions/signal-mobile-menus.md).

use crate::config::BotRole;

pub fn help_menu(role: BotRole) -> &'static str {
    match role {
        BotRole::Transcription => HELP_TRANSCRIPTION,
        BotRole::Translation => HELP_HUB,
    }
}

/// Same commands as [`help_menu`], with short explanations and blank-line breaks.
pub fn explain_menu(role: BotRole) -> &'static str {
    match role {
        BotRole::Transcription => EXPLAIN_TRANSCRIPTION,
        BotRole::Translation => EXPLAIN_HUB,
    }
}

pub fn thread_help_menu() -> &'static str {
    HELP_THREAD
}

pub fn thread_explain_menu() -> &'static str {
    EXPLAIN_THREAD
}

pub fn translation_threads_menu() -> &'static str {
    TRANSLATION_THREADS_MENU
}

pub fn translation_in_chat_menu(translate_all_enabled: bool) -> &'static str {
    if translate_all_enabled {
        TRANSLATION_IN_CHAT_MENU
    } else {
        TRANSLATION_IN_CHAT_MENU_AUTO_DISABLED
    }
}

/// Legacy `!translation` redirect naming the two product menus.
pub fn translation_split_redirect() -> &'static str {
    TRANSLATION_SPLIT_REDIRECT
}

pub fn transcription_unavailable() -> &'static str {
    TRANSCRIPTION_UNAVAILABLE
}

pub fn transcription_invited() -> &'static str {
    TRANSCRIPTION_INVITED
}

pub fn transcription_group_only() -> &'static str {
    TRANSCRIPTION_GROUP_ONLY
}

pub fn privacy_menu(role: BotRole) -> &'static str {
    match role {
        BotRole::Transcription => PRIVACY_TRANSCRIPTION,
        BotRole::Translation => PRIVACY_TRANSLATION,
    }
}

/// Exact command match (avoids `!translation` matching `!translation-on`).
pub fn is_exact_command(text: &str, command: &str) -> bool {
    text.trim() == command
}

const HELP_TRANSCRIPTION: &str = r#"Voice transcription

Voice notes in this chat are transcribed to text (Whisper, inside the TEE).

!transcription
  This menu
!transcribe-on / !transcribe-off
  Toggle auto transcription
!transcribe
  Quote a voice note to transcribe
!privacy
  Privacy & TEE
!explain
!help"#;

const HELP_HUB: &str = r#"Sigstack

!translation-threads
!translation-in-chat
!transcription
!privacy
!explain
!help"#;

const HELP_THREAD: &str = r#"Language Thread

!rename <name>
  Change this group's name
!leave
  Leave this Language Thread
!explain
!help"#;

const EXPLAIN_HUB: &str = r#"Sigstack

!translation-threads
  Language Threads — multilingual main chat + language sidecars

!translation-in-chat
  In-chat translation — auto or quote-translate in this group

!transcription
  Voice transcription — pair/open the transcription bot

!privacy
  Privacy & TEE — attestation via !verify

!explain
  This menu (commands with explanations)

!help
  Compact command list"#;

const EXPLAIN_TRANSCRIPTION: &str = r#"Voice transcription

Voice notes in this chat are transcribed to text (Whisper, inside the TEE).

!transcription
  This product menu

!transcribe-on / !transcribe-off
  Toggle auto transcription

!transcribe
  Quote a voice note to transcribe it

!privacy
  Privacy & TEE — attestation via !verify

!explain
  This menu (commands with explanations)

!help
  Compact command list"#;

const EXPLAIN_THREAD: &str = r#"Language Thread

!rename <name>
  Change this Language Thread's group name

!leave
  Leave this Language Thread

!explain
  This menu (commands with explanations)

!help
  Compact command list"#;

const TRANSLATION_THREADS_MENU: &str = r#"Language Threads

Multilingual main + language sidecars.
Join/create a Language Thread

!list-langs
!translate-me-thread <lang>
  e.g. !translate-me-thread es

!disable-threads
(to enable in-chat translation)

!help"#;

const TRANSLATION_IN_CHAT_MENU: &str = r#"In-chat translation

Stay in this thread; auto or quote one message.

Translate everyone's msgs:

!list-langs
!translate-all-on <lang1> <lang2>
!translate-all-off
  e.g. !translate-all-on fr zh

Translate your msgs:

!translate-me-on <lang1> <lang2>
!translate-me-off
  e.g. !translate-me-on ru ar

Translate per msg:

!translate <lang>

!disable-in-chat
(to enable threads)

!help"#;

const TRANSLATION_IN_CHAT_MENU_AUTO_DISABLED: &str = r#"In-chat translation

Auto-translate is disabled on this bot (!translate-all-on).

!translate <lang>
  Reply to a message

!help"#;

const TRANSLATION_SPLIT_REDIRECT: &str = r#"Translation has two menus:

!translation-threads
!translation-in-chat

!help"#;

const TRANSCRIPTION_UNAVAILABLE: &str = r#"Voice transcription is currently unavailable.

The transcription bot is not paired with this group yet. Meanwhile, try translation:

!translation-threads
!translation-in-chat

!help"#;

const TRANSCRIPTION_INVITED: &str = r#"Invited the transcription bot to this group.

Accept the Signal invite on that number, then send !transcription again (the transcription bot will answer with its menu).

!help"#;

const TRANSCRIPTION_GROUP_ONLY: &str = r#"Voice transcription pairing works in a Signal group.

Add both bots to a group, then send !transcription there.

!help"#;

const PRIVACY_TRANSCRIPTION: &str = r#"**Sigstack transcription** (Private & Verifiable)

**TEE Commands:**
!verify <challenge>
  Get TEE attestation with your challenge

**Privacy:**
Voice notes are decrypted by Signal CLI inside this TEE and transcribed with Whisper in the same CVM. Text transcripts are posted back to Signal.

Neither the bot operator nor the host can read decrypted audio or text in TEE memory.

Pair with the translation bot in the same group if you also want translation."#;

const PRIVACY_TRANSLATION: &str = r#"**Sigstack translation** (Private & Verifiable)

**TEE Commands:**
!verify <challenge>
  Get TEE attestation with your challenge

**Verification:**
`!verify my-random-text` to get cryptographic proof this bot runs in a TEE. Your challenge is embedded in the TDX quote.

**Privacy:**
Messages are end-to-end encrypted via Signal, processed in a verified TEE (Intel TDX), and translated via NEAR AI Cloud private inference (NVIDIA GPU TEE).

Voice transcription is a separate bot/CVM. This bot only acts on text (including transcripts posted by the transcription bot).

Neither the bot operator nor NEAR AI can read your messages in plaintext outside the TEEs.

!help"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_translation_is_hub() {
        let h = help_menu(BotRole::Translation);
        assert!(h.contains("!translation-threads"));
        assert!(h.contains("!translation-in-chat"));
        assert!(h.contains("!transcription"));
        assert!(h.contains("!privacy"));
        assert!(h.contains("!explain"));
        assert!(!h.contains("Language Threads\n"));
        assert!(!h.contains("In-chat translation\n"));
        assert!(!h.contains("  "));
        assert!(!h.contains("!ask"));
        assert!(!h.contains("!models"));
    }

    #[test]
    fn explain_hub_has_breaks_and_descriptions() {
        let h = explain_menu(BotRole::Translation);
        assert!(h.contains("!translation-threads\n  "));
        assert!(h.contains("!translation-in-chat\n  "));
        assert!(h.contains("!transcription\n  "));
        assert!(h.contains("!privacy\n  "));
        assert!(h.contains("!explain\n  "));
        assert!(h.contains("!help\n  "));
        assert!(h.contains("\n\n!translation-in-chat"));
        assert!(h.contains("attestation via !verify"));
        assert!(!h.contains("!verify <challenge>"));
    }

    #[test]
    fn thread_menu_has_leave_not_subscribe() {
        let h = thread_help_menu();
        assert!(h.contains("!leave"));
        assert!(h.contains("!rename"));
        assert!(!h.contains("!translate-me-thread"));
        assert!(!h.contains("!translate-me-on"));
    }

    #[test]
    fn threads_menu_lists_thread_commands() {
        let h = translation_threads_menu();
        assert!(h.contains("!translate-me-thread <lang>"));
        assert!(h.contains("!disable-threads"));
        assert!(h.contains("!list-langs"));
        assert!(!h.contains("!leave"));
        assert!(!h.contains("!translate-all-on"));
    }

    #[test]
    fn in_chat_menu_lists_auto_commands() {
        let h = translation_in_chat_menu(true);
        assert!(h.contains("!translate-all-on <lang1> <lang2>"));
        assert!(h.contains("!translate-me-on <lang1> <lang2>"));
        assert!(h.contains("!disable-in-chat"));
        assert!(h.contains("!translate <lang>"));
        assert!(!h.contains("!translate-me-thread"));
        assert!(!h.contains("!models"));
    }

    #[test]
    fn in_chat_menu_auto_disabled_hides_on_commands() {
        let h = translation_in_chat_menu(false);
        assert!(h.contains("Auto-translate is disabled"));
        assert!(!h.contains("!translate-all-on <lang1>"));
        assert!(h.contains("!translate <lang>"));
    }

    #[test]
    fn exact_command_does_not_match_prefixed() {
        assert!(is_exact_command("!translation", "!translation"));
        assert!(!is_exact_command("!translation-on es en", "!translation"));
        assert!(is_exact_command("!in-chat", "!in-chat"));
        assert!(!is_exact_command("!in-chat-extra", "!in-chat"));
        assert!(is_exact_command(
            "!translation-threads",
            "!translation-threads"
        ));
    }

    #[test]
    fn help_transcription_covers_voice() {
        let h = help_menu(BotRole::Transcription);
        assert!(h.contains("!transcribe"));
        assert!(!h.contains("!ask"));
        assert!(!h.contains("!translate-me-on"));
        assert!(!h.contains("!verify"));
        assert!(h.contains("!transcribe-on / !transcribe-off\n  "));
    }

    #[test]
    fn privacy_menus_cover_roles() {
        let transcription = privacy_menu(BotRole::Transcription);
        assert!(transcription.contains("Sigstack transcription"));
        assert!(transcription.contains("!verify <challenge>\n  "));
        let translation = privacy_menu(BotRole::Translation);
        assert!(translation.contains("Sigstack translation"));
        assert!(translation.contains("!verify <challenge>\n  "));
        assert!(!translation.contains("!models"));
    }

    #[test]
    fn product_menus_omit_verify() {
        assert!(!translation_threads_menu().contains("!verify"));
        assert!(!translation_in_chat_menu(true).contains("!verify"));
        assert!(!translation_in_chat_menu(false).contains("!verify"));
        assert!(!help_menu(BotRole::Translation).contains("!verify"));
        assert!(!thread_help_menu().contains("!verify"));
    }

    #[test]
    fn transcription_unavailable_offers_translation() {
        let m = transcription_unavailable();
        assert!(m.contains("unavailable"));
        assert!(m.contains("!translation-threads"));
        assert!(m.contains("!translation-in-chat"));
    }
}
