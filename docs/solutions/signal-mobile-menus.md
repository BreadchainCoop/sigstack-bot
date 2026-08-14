# Signal mobile menus

## Problem

Long one-liners like `- !translate-me-on <lang> — join/create a Language Thread (from main)` wrap mid-phrase on mobile Signal and look broken.

## Standard

Source of truth for bot command menus: [`crates/signal-bot/src/commands/menu_locale.rs`](../../crates/signal-bot/src/commands/menu_locale.rs).

Menus are **English-only** for now; multi-language UI is deferred.

1. **Title** on its own line (product or hub name).
2. **Section header** (optional) + optional one-line blurb.
3. **Each command** on its own line. No `cmd — desc` on one line.
4. Prefer plain `!command` lines over `- !command` bullets.
5. **Hub / nav lists** (e.g. main `!help` on Bread Bot): commands only — skip indented descriptions when they would just restate the command name.
6. **Product / how-to lists** (e.g. `!translation-in-chat`, transcription toggles): put a short description on the next line, indented with two spaces (aim ≤~40 chars).
7. **`!help` footer** is the bare command — no “Main menu” / “Show this menu” line (`!help` is implicit).
8. Prose blocks (privacy explanations, invite/status messages) stay paragraphs; only **command lists** use the forms above.

Hub shape:

```text
Bread Bot

!translation-threads
!translation-in-chat
!transcription
!privacy
!help-transcription
!info
!help
```

`!info` returns the same commands with a short indented description under each and a blank line between entries.

Product feature guides (`!help-threads`, `!help-in-chat`, `!help-transcription`) are short prose (use case + typical flow), not stacked command lists.

Product shape:

```text
Title

Section header
Optional one-line blurb.

!command <args>
  Short description
!other-command
  Short description
!help
```

Language Thread sidecars use `!commands` for the thread menu (rename / leave / info). Hub `!help` always returns the Bread Bot menu, including when sent from a sidecar.

Voice transcription uses `!transcription` for its product menu (not hub `!help`). Hub `!help` / `!info` / `!privacy` and `!transcription` all live on the same bot. See [two-cvm-architecture.md](../two-cvm-architecture.md).

## When adding menus

Apply this layout in `menu_locale.rs` for any new `!help` / product / privacy command list. Do not reintroduce `!cmd — long description` one-liners.
