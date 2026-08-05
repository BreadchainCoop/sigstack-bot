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

/// Hub descriptive menu (translation bot only; transcription uses `!transcription`).
pub fn info_menu(role: BotRole) -> &'static str {
    match role {
        BotRole::Translation => INFO_HUB,
        BotRole::Transcription => unreachable!("transcription bot does not register !info"),
    }
}

pub fn thread_help_menu() -> &'static str {
    HELP_THREAD
}

pub fn thread_info_menu() -> &'static str {
    INFO_THREAD
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

/// How Language Threads works (use case + flow).
pub fn help_threads_guide() -> &'static str {
    HELP_THREADS_GUIDE
}

/// How in-chat translation works (use case + flow).
pub fn help_in_chat_guide() -> &'static str {
    HELP_IN_CHAT_GUIDE
}

/// How voice transcription works (use case + flow).
pub fn help_transcription_guide() -> &'static str {
    HELP_TRANSCRIPTION_GUIDE
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

/// Exact privacy menu trigger per bot role (avoids dual-bot `!privacy` collisions).
pub fn privacy_command(role: BotRole) -> &'static str {
    match role {
        BotRole::Translation => "!privacy-translation",
        BotRole::Transcription => "!privacy-transcription",
    }
}

/// Exact command match (avoids `!translation` matching `!translation-on`).
pub fn is_exact_command(text: &str, command: &str) -> bool {
    text.trim() == command
}

pub fn is_exact_command_any(text: &str, commands: &[&str]) -> bool {
    let t = text.trim();
    commands.contains(&t)
}

const TRANSLATION_THREADS_MENU_COMMANDS: &[&str] = &[
    "!translation-threads",
    "!translate-threads",
    "!translate-thread",
    "!translation-thread",
];

const TRANSLATION_IN_CHAT_MENU_COMMANDS: &[&str] = &["!translation-in-chat", "!translate-in-chat"];

/// Product hub menu for Language Threads (canonical + common typos).
pub fn is_translation_threads_menu_command(text: &str) -> bool {
    is_exact_command_any(text, TRANSLATION_THREADS_MENU_COMMANDS)
}

/// Product hub menu for in-chat translation (canonical + common typos).
pub fn is_translation_in_chat_menu_command(text: &str) -> bool {
    is_exact_command_any(text, TRANSLATION_IN_CHAT_MENU_COMMANDS)
}

const HELP_TRANSCRIPTION: &str = r#"Voice transcription

!transcribe-on
  Turn auto transcription on
!transcribe-off
  Turn auto transcription off
!transcribe
  Quote a voice note to transcribe

examples:
   !transcribe (reply quoting a voice note)

Voice notes become quote-reply transcripts (Whisper, inside this bot's TEE).

!help-transcription
!privacy-transcription"#;

const HELP_HUB: &str = r#"--Bread Bot--

MENUS:
!translation-threads
!translation-in-chat
!transcription

GUIDES:
!help-threads
!help-in-chat
!help-transcription

OTHER:
!info
!privacy-translation
!help"#;

const HELP_THREAD: &str = r#"Language Thread

!rename <name>
  Change this group's name
!leave
  Leave this Language Thread
!info
!help"#;

const INFO_HUB: &str = r#"--Bread Bot--

!translation-threads
  Language Threads — multilingual main chat + language sidecars

!translation-in-chat
  In-chat translation — auto or quote-translate in this group

!transcription
  Voice transcription — pair/open the transcription bot

!privacy-translation
  Privacy & TEE — attestation via !verify

!help-transcription
  How voice transcription works

!info
  This menu (commands with descriptions)

!help
  Compact command list"#;

const INFO_THREAD: &str = r#"Language Thread

!rename <name>
  Change this Language Thread's group name

!leave
  Leave this Language Thread

!info
  This menu (commands with descriptions)

!help
  Compact command list"#;

const TRANSLATION_THREADS_MENU: &str = r#"Join/Create Language Thread

!list-langs
!translate-me-thread <lang>
!help-threads

example:
   !translate-me-thread es

Unlimited threads are supported. Main chat stays multilingual and threads relay messages between them. Once you join a thread, just read/write in from that thread.

!enable-in-chat (disable threads)
!help"#;

const TRANSLATION_IN_CHAT_MENU: &str = r#"In-chat Translation

!list-langs
!translate-all-on <lang1> <lang2>
!translate-all-off
!translate-me-on <lang1> <lang2>
!translate-me-off
!translate <lang> (as reply)
!help-in-chat

examples:
   !translate-all-on fr zh
   !translate-me-on ru ar
   !translate es

!enable-threads (disable in-chat)
!help"#;

const TRANSLATION_IN_CHAT_MENU_AUTO_DISABLED: &str = r#"In-chat translation

Auto-translate is disabled on this bot (!translate-all-on).

!translate <lang>
  Reply to a message

!help-in-chat
!help"#;

const HELP_THREADS_GUIDE: &str = r#"Language Threads — how it works

Use when people need monolingual lanes, but organizers still want one shared main chat.

How it works:
- Main group stays multilingual (everyone can post in any language).
- Each language gets a sidecar Signal group ("Language Thread").
- Messages bridge: main ↔ threads (relay same language, translate otherwise).

Typical use:
1. In main, send !list-langs then !translate-me-thread es
2. Accept the sidecar invite
3. Read/write in that thread; the bot bridges with main and other threads
4. !leave from a thread to leave it
5. From main, !enable-in-chat tears down threads if you want in-chat auto instead

Language Threads and in-chat auto cannot run at the same time.

Commands: !translation-threads
!help"#;

const HELP_IN_CHAT_GUIDE: &str = r#"In-chat translation — how it works

Use when everyone stays in one Signal group and wants bilingual (or quote) translation there.

How it works:
- No sidecar groups — replies stay in this chat as quote-replies.
- Group-wide: !translate-all-on es en auto-translates messages between that pair.
- Personal: !translate-me-on es en auto-translates only your messages.
- One-off: reply to a message with !translate <lang>

Typical use:
1. Pick two languages (!list-langs)
2. !translate-all-on es en (or !translate-me-on for just you)
3. Chat normally; the bot quote-replies translations
4. !translate-all-off / !translate-me-off to stop
5. From this group, !enable-threads clears in-chat auto if you want Language Threads instead

In-chat auto and Language Threads cannot run at the same time.

Commands: !translation-in-chat
!help"#;

const HELP_TRANSCRIPTION_GUIDE: &str = r#"Voice transcription — how it works

Use when people send voice notes and you want text in the same Signal chat.

This bot runs Whisper in its own Phala CVM/TEE. The Bread Bot translation bot is a separate CVM — hub menus (!help, !info) and translation attestation live on that number. Transcription attestation is via !privacy-transcription / !verify on this bot.

How it works:
- Auto mode (default on): inbound voice notes become quote-reply transcripts.
- Manual: quote a voice note and send !transcribe
- Toggle with !transcribe-on / !transcribe-off

Typical use:
1. Add both bots to the group (translation can invite via !transcription if PEER_PHONE is set)
2. Accept the invite on the transcription number
3. Send !transcription on the transcription bot for its command menu
4. Send a voice note — you get a 📝 Transcript: reply
5. With the translation bot in the group, that transcript can also be auto-translated

Commands: !transcription
Privacy / TEE (this CVM): !privacy-transcription (!verify)"#;

const TRANSLATION_SPLIT_REDIRECT: &str = r#"Translation has two menus:

!translation-threads
!translation-in-chat

!help"#;

const TRANSCRIPTION_UNAVAILABLE: &str = r#"Voice transcription is currently unavailable.

The transcription bot is not paired with this group yet. Meanwhile, try translation:

!translation-threads
!translation-in-chat

!help-transcription
!help"#;

const TRANSCRIPTION_INVITED: &str = r#"Invited the transcription bot to this group.

Accept the Signal invite on that number, then send !transcription again (the transcription bot will answer with its menu).

!help"#;

const TRANSCRIPTION_GROUP_ONLY: &str = r#"Voice transcription pairing works in a Signal group.

Add both bots to a group, then send !transcription there.

!help"#;

const PRIVACY_TRANSCRIPTION: &str = r#"**Bread Bot transcription** (Private & Verifiable)

**TEE Commands:**
!verify <challenge>
  Get TEE attestation with your challenge

**Privacy:**
Voice notes are decrypted by Signal CLI inside this TEE and transcribed with Whisper in the same CVM. Text transcripts are posted back to Signal.

Neither the bot operator nor the host can read decrypted audio or text in TEE memory.

Pair with the translation bot in the same group if you also want translation."#;

const PRIVACY_TRANSLATION: &str = r#"**Bread Bot translation** (Private & Verifiable)

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
        assert!(h.contains("!privacy-translation"));
        assert!(h.contains("!info"));
        assert!(!h.contains("Language Threads\n"));
        assert!(!h.contains("In-chat translation\n"));
        assert!(!h.contains("  "));
        assert!(!h.contains("!ask"));
        assert!(!h.contains("!models"));
    }

    #[test]
    fn info_hub_has_breaks_and_descriptions() {
        let h = info_menu(BotRole::Translation);
        assert!(h.contains("!translation-threads\n  "));
        assert!(h.contains("!translation-in-chat\n  "));
        assert!(h.contains("!transcription\n  "));
        assert!(h.contains("!privacy-translation\n  "));
        assert!(h.contains("!info\n  "));
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
        assert!(h.contains("!enable-in-chat"));
        assert!(h.contains("!help-threads"));
        assert!(h.contains("!list-langs"));
        assert!(!h.contains("!leave"));
        assert!(!h.contains("!translate-all-on"));
    }

    #[test]
    fn in_chat_menu_lists_auto_commands() {
        let h = translation_in_chat_menu(true);
        assert!(h.contains("!translate-all-on <lang1> <lang2>"));
        assert!(h.contains("!translate-me-on <lang1> <lang2>"));
        assert!(h.contains("!enable-threads"));
        assert!(h.contains("!help-in-chat"));
        assert!(h.contains("!translate <lang>"));
        assert!(!h.contains("!translate-me-thread"));
        assert!(!h.contains("!models"));
    }

    #[test]
    fn feature_guides_cover_use_cases() {
        let threads = help_threads_guide();
        assert!(threads.contains("Language Threads"));
        assert!(threads.contains("!translate-me-thread"));
        assert!(threads.contains("sidecar"));
        let in_chat = help_in_chat_guide();
        assert!(in_chat.contains("In-chat translation"));
        assert!(in_chat.contains("!translate-all-on"));
        assert!(in_chat.contains("quote"));
        let transcription = help_transcription_guide();
        assert!(transcription.contains("Voice transcription"));
        assert!(transcription.contains("Whisper"));
        assert!(transcription.contains("!transcribe"));
        assert!(transcription.contains("separate CVM"));
        assert!(transcription.contains("!privacy-transcription"));
        assert!(!transcription.trim_end().ends_with("!help"));
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
    fn translation_threads_menu_command_aliases() {
        assert!(is_translation_threads_menu_command("!translation-threads"));
        assert!(is_translation_threads_menu_command("!translate-threads"));
        assert!(is_translation_threads_menu_command("!translate-thread"));
        assert!(is_translation_threads_menu_command("!translation-thread"));
        assert!(is_translation_threads_menu_command(
            "  !translation-threads  "
        ));
        assert!(!is_translation_threads_menu_command(
            "!translation-on es en"
        ));
        assert!(!is_translation_threads_menu_command(
            "!translation-threads-extra"
        ));
    }

    #[test]
    fn translation_in_chat_menu_command_aliases() {
        assert!(is_translation_in_chat_menu_command("!translation-in-chat"));
        assert!(is_translation_in_chat_menu_command("!translate-in-chat"));
        assert!(is_translation_in_chat_menu_command(
            "  !translate-in-chat  "
        ));
        assert!(!is_translation_in_chat_menu_command(
            "!translation-on es en"
        ));
        assert!(!is_translation_in_chat_menu_command(
            "!translation-in-chat-extra"
        ));
    }

    #[test]
    fn help_transcription_covers_voice() {
        let h = help_menu(BotRole::Transcription);
        assert!(h.contains("!transcribe"));
        assert!(h.contains("!transcribe-on\n  "));
        assert!(h.contains("!transcribe-off\n  "));
        assert!(h.contains("!help-transcription"));
        assert!(h.contains("!privacy-transcription"));
        assert!(!h.contains("!ask"));
        assert!(!h.contains("!translate-me-on"));
        assert!(!h.contains("!verify"));
        assert!(!h.contains("!info"));
        assert!(!h.trim_end().ends_with("!help"));
    }

    #[test]
    fn privacy_menus_cover_roles() {
        let transcription = privacy_menu(BotRole::Transcription);
        assert!(transcription.contains("Bread Bot transcription"));
        assert!(transcription.contains("!verify <challenge>\n  "));
        let translation = privacy_menu(BotRole::Translation);
        assert!(translation.contains("Bread Bot translation"));
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
