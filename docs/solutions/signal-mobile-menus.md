# Signal mobile menus

## Problem

Long one-liners like `- !translate-me-on <lang> — join/create a Language Thread (from main)` wrap mid-phrase on mobile Signal and look broken.

## Standard

Source of truth for bot command menus: [`crates/signal-bot/src/commands/menu_locale.rs`](../../crates/signal-bot/src/commands/menu_locale.rs).

1. **Title** on its own line (product or hub name).
2. **Section header** (optional) + optional one-line blurb.
3. **Each command** on its own line; **description** on the next line, indented with two spaces. No `cmd — desc` on one line.
4. Prefer plain `!command` lines over `- !command` bullets.
5. Keep descriptions short (aim ≤~40 chars when possible).
6. **Footer** nav may stay compact (`!help — Main menu`, `Also: !models · !verify <challenge>`).
7. Prose blocks (privacy explanations, invite/status messages) stay paragraphs; only **command lists** use the stacked form.
8. EN and ES stay structural twins.

Canonical shape:

```text
Title

Section header
Optional one-line blurb.

!command <args>
  Short description
!other-command
  Short description

Also: !models · !verify <challenge>
!help — Main menu
```

## When adding menus

Apply this layout in `menu_locale.rs` for any new `!help` / product / privacy command list. Do not reintroduce `!cmd — long description` one-liners.
