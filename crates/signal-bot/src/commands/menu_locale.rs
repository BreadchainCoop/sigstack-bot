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

pub fn thread_help_menu() -> &'static str {
    HELP_THREAD
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
!help
  Show this menu
!verify <challenge>
  TEE attestation"#;

const HELP_HUB: &str = r#"Sigstack

!translation-threads
  Language Threads
!translation-in-chat
  In-chat translation
!transcription
  Voice transcription
!privacy
  Privacy & TEE
!help
  Show this menu"#;

const HELP_THREAD: &str = r#"Language Thread

!rename <name>
  Change this group's name
!leave
  Leave this Language Thread
!help
  Show this menu"#;

const TRANSLATION_THREADS_MENU: &str = r#"Language Threads

Multilingual main + language sidecars.

!translate-me-thread <lang>
  Join/create a Language Thread (from main)
!disable-threads
  Turn off Language Threads for this group
!list-langs
  Language codes

!verify <challenge>
  TEE attestation
!help
  Main menu"#;

const TRANSLATION_IN_CHAT_MENU: &str = r#"In-chat translation

Stay in this thread; auto or quote one message.

!translate-all-on <lang1> <lang2>
  e.g. !translate-all-on es en
!translate-all-off
  Stop group-wide auto-translate
!translate-me-on <lang1> <lang2>
  Auto-translate your messages only
!translate-me-off
  Stop your personal auto-translate
!disable-in-chat
  Turn off all in-chat auto-translate
!translate <lang>
  Reply to a message

!verify <challenge>
  TEE attestation
!help
  Main menu"#;

const TRANSLATION_IN_CHAT_MENU_AUTO_DISABLED: &str = r#"In-chat translation

Auto-translate is disabled on this bot (!translate-all-on).

!translate <lang>
  Reply to a message

!verify <challenge>
  TEE attestation
!help
  Main menu"#;

const TRANSLATION_SPLIT_REDIRECT: &str = r#"Translation has two menus:

!translation-threads
  Language Threads
!translation-in-chat
  In-chat translation

!help
  Main menu"#;

const TRANSCRIPTION_UNAVAILABLE: &str = r#"Voice transcription is currently unavailable.

The transcription bot is not paired with this group yet. Meanwhile, try translation:

!translation-threads
  Language Threads
!translation-in-chat
  In-chat translation

!help
  Main menu"#;

const TRANSCRIPTION_INVITED: &str = r#"Invited the transcription bot to this group.

Accept the Signal invite on that number, then send !transcription again (the transcription bot will answer with its menu).

!help
  Main menu"#;

const TRANSCRIPTION_GROUP_ONLY: &str = r#"Voice transcription pairing works in a Signal group.

Add both bots to a group, then send !transcription there.

!help
  Main menu"#;

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

!help
  Main menu"#;

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
        assert!(!h.contains("!ask"));
        assert!(!h.contains("!models"));
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
        assert!(h.contains("!transcribe-on / !transcribe-off\n  "));
    }

    #[test]
    fn privacy_menus_cover_roles() {
        let transcription = privacy_menu(BotRole::Transcription);
        assert!(transcription.contains("Sigstack transcription"));
        assert!(transcription.contains("!verify <challenge>\n  "));
        let translation = privacy_menu(BotRole::Translation);
        assert!(translation.contains("Sigstack translation"));
        assert!(!translation.contains("!models"));
    }

    #[test]
    fn transcription_unavailable_offers_translation() {
        let m = transcription_unavailable();
        assert!(m.contains("unavailable"));
        assert!(m.contains("!translation-threads"));
        assert!(m.contains("!translation-in-chat"));
    }
}
