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

pub fn translation_products_menu(translate_all_enabled: bool) -> &'static str {
    if translate_all_enabled {
        TRANSLATION_MENU
    } else {
        TRANSLATION_MENU_AUTO_DISABLED
    }
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

!translation
  Translation
!transcription
  Voice transcription
!privacy
  Privacy & TEE
!help
  Show this menu"#;

const HELP_THREAD: &str = r#"Language Thread

!rename <name>
  Change this group's name
!translate-me-off
  Leave this Language Thread
!help
  Show this menu"#;

const TRANSLATION_MENU: &str = r#"Translation

Language Threads (recommended)
Multilingual main + language sidecars.

!translate-me-on <lang>
  Join/create a Language Thread (from main)
!translate-me-off
  Leave your Language Thread
!list-langs
  Language codes

In-chat (same group only)
Stay in this thread; auto or quote one message.

!translate-on <lang1> <lang2>
  e.g. !translate-on es en
!translate-off
  Stop auto-translate
!translate <lang>
  Reply to a message

!verify <challenge>
  TEE attestation
!help
  Main menu"#;

const TRANSLATION_MENU_AUTO_DISABLED: &str = r#"Translation

Language Threads (recommended)
Multilingual main + language sidecars.

!translate-me-on <lang>
  Join/create a Language Thread (from main)
!translate-me-off
  Leave your Language Thread
!list-langs
  Language codes

In-chat (same group only)
Auto-translate is disabled on this bot (!translate-on).

!translate <lang>
  Reply to a message

!verify <challenge>
  TEE attestation
!help
  Main menu"#;

const TRANSCRIPTION_UNAVAILABLE: &str = r#"Voice transcription is currently unavailable.

The transcription bot is not paired with this group yet. Meanwhile, try translation:

!translation
  Translation

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
        assert!(h.contains("!translation"));
        assert!(h.contains("!transcription"));
        assert!(h.contains("!privacy"));
        assert!(!h.contains("!translate-me-on"));
        assert!(!h.contains("!transcribe-on"));
        assert!(
            h.contains("!translation\n  Translation"),
            "hub commands should use stacked layout"
        );
        assert!(!h.contains("!set-en"));
        assert!(!h.contains("!set-es"));
        assert!(h.contains("!help\n  Show this menu"));
        assert!(!h.contains("!translation —"));
        assert!(!h.contains("!help —"));
    }

    #[test]
    fn thread_help_covers_rename() {
        let h = thread_help_menu();
        assert!(h.contains("!rename <name>"));
        assert!(h.contains("!translate-me-off"));
        assert!(h.contains("!help\n  Show this menu"));
        assert!(!h.contains("!set-en"));
        assert!(!h.contains("!translate-me-on"));
    }

    #[test]
    fn translation_menu_leads_with_language_threads() {
        let h = translation_products_menu(true);
        assert!(h.contains("Language Threads (recommended)"));
        assert!(h.contains("!translate-me-on"));
        assert!(h.contains("!translate-me-off"));
        assert!(h.contains("!translate-on"));
        assert!(h.contains("!translate <lang>"));
        assert!(!h.contains("!parallel"));
        assert!(!h.contains("!in-chat"));
        let lt = h.find("Language Threads").expect("lt");
        let in_chat = h.find("In-chat").expect("in-chat section");
        assert!(
            lt < in_chat,
            "Language Threads should appear before In-chat"
        );
        assert!(
            h.contains("!translate-me-on <lang>\n  "),
            "translation menu should use stacked layout"
        );
        assert!(!h.contains("!translate-me-on <lang> —"));
    }

    #[test]
    fn translation_menu_auto_disabled_hides_translate_on() {
        let h = translation_products_menu(false);
        assert!(h.contains("!translate-me-on"));
        assert!(h.contains("Auto-translate is disabled"));
        assert!(!h.contains("!translate-on <lang1>"));
        assert!(h.contains("!translate <lang>"));
        assert!(h.contains("!translate <lang>\n  "));
    }

    #[test]
    fn exact_command_does_not_match_prefixed() {
        assert!(is_exact_command("!translation", "!translation"));
        assert!(!is_exact_command("!translation-on es en", "!translation"));
        assert!(is_exact_command("!in-chat", "!in-chat"));
        assert!(!is_exact_command("!in-chat-extra", "!in-chat"));
    }

    #[test]
    fn help_transcription_covers_voice() {
        let h = help_menu(BotRole::Transcription);
        assert!(h.contains("!transcribe"));
        assert!(!h.contains("!ask"));
        assert!(!h.contains("!translate-me-on"));
        assert!(h.contains("!transcribe-on / !transcribe-off\n  "));
        assert!(!h.contains("!transcribe-on / !transcribe-off —"));
    }

    #[test]
    fn privacy_menus_cover_roles() {
        let transcription = privacy_menu(BotRole::Transcription);
        assert!(transcription.contains("Sigstack transcription"));
        assert!(transcription.contains("!verify <challenge>\n  "));
        assert!(!transcription.contains("!verify <challenge> -"));
        let translation = privacy_menu(BotRole::Translation);
        assert!(translation.contains("Sigstack translation"));
        assert!(translation.contains("!verify <challenge>\n  "));
        assert!(!translation.contains("!models"));
    }

    #[test]
    fn transcription_unavailable_offers_translation() {
        let m = transcription_unavailable();
        assert!(m.contains("unavailable"));
        assert!(m.contains("!translation"));
        assert!(m.contains("!translation\n  Translation"));
        assert!(!m.contains("!translation —"));
    }
}
