/**
 * Product Signal menus for the marketing site.
 * Keep in sync with crates/signal-bot/src/commands/menu_locale.rs
 * (TRANSLATION_THREADS_MENU, TRANSLATION_IN_CHAT_MENU, HELP_TRANSCRIPTION).
 */

export const threadsMenu = `Join/Create Language Thread

!list-langs
!translate-me-thread <lang>
!translate-me-thread <main> <thread>
!help-threads

examples:
   !translate-me-thread es
   !translate-me-thread es en

Language Threads: unlimited sidecars; main stays multilingual.
Bilingual Threads: exactly two languages, one sidecar; each room is assigned a language and the bot translates both ways.

!enable-in-chat (disable threads)
!help`;

export const inChatMenu = `In-chat Translation

!list-langs-in-chat
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

Quote !translate accepts any Language Threads language (!list-langs).
!enable-threads (disable in-chat)
!help`;

export const transcriptionMenu = `Voice Transcription

AUTO:
!transcribe-on
!transcribe-off

PER MSG QUOTE REPLY:
!transcribe

!help-transcription`;
