# In-chat (group) translation

Status: **MVP implemented** on the translation bot.

One **bilingual** Signal group (e.g. English + Spanish). The bot detects which side of the pair a message is on and quote-replies with the other language in the **same** main thread.

Distinct from [Language Threads](language-threads.md) (multilingual main + N sidecars). In-chat stays in one Signal group; Language Threads creates sidecar groups. **In-chat auto and Language Threads are mutually exclusive** — enabling one while the other is active refuses with a switch path (`!disable-threads` / `!disable-in-chat`).

## Setup

Menus: `!help` → `!translation-in-chat` (or legacy `!in-chat` / `!translation` redirect).

### Group-wide

```text
!translate-all-on es en
```

- Stores a bilingual pair for this group (`lang_a` ↔ `lang_b`)
- Order does not matter for detection (either side maps to the other)
- Aliases: `!translate-on`, `!translation-on`, `!translation-all-on`

Stop group-wide only:

```text
!translate-all-off
```

### Personal (per subscriber)

```text
!translate-me-on es en
```

- Auto-translates **that user’s** messages only (quote-reply in the same chat)
- Other members’ messages are unchanged unless group-wide is also on

Stop personal:

```text
!translate-me-off
```

Clear **all** in-chat auto (group-wide + every personal), and apply a pending Language Threads subscribe if one was refused earlier:

```text
!disable-in-chat
```

## Behavior

| Mode | How | Effect |
|------|-----|--------|
| **Group auto** | `!translate-all-on` active | Every non-command group text: detect → if in pair → NEAR translate → quote-reply `{flag} {translation}` |
| **Personal auto** | `!translate-me-on` for author | Same as group auto, but only for that author’s messages. Personal pair wins over group-wide for that author (one quote-reply max). |
| **Manual** | Reply with `!translate <lang>` | Translate only that quoted message (always allowed) |

Not dual-post: the original stays as the human message; the bot only quote-replies the translation.

Skip when language is undetected or not in the pair. Rate-limited per group (`TRANSLATE_ALL__MAX_MESSAGES_PER_MINUTE`).

Voice notes: the **transcription** bot posts a transcript in-group; with auto-translate on, the **translation** bot then intercepts that text like any other message.

## Key code

| Area | Path |
|------|------|
| Auto on/off + intercept | [`crates/signal-bot/src/commands/translate_all.rs`](../crates/signal-bot/src/commands/translate_all.rs) |
| Quote `!translate` | [`crates/signal-bot/src/commands/translate.rs`](../crates/signal-bot/src/commands/translate.rs) |
| Detect / format helpers | [`crates/signal-bot/src/commands/translate_service.rs`](../crates/signal-bot/src/commands/translate_service.rs) |
| Prefs (`GroupTranslateMode`, `translate_members`) | [`crates/signal-bot/src/group_preferences_store.rs`](../crates/signal-bot/src/group_preferences_store.rs) |
| Menus | [`crates/signal-bot/src/commands/menu_locale.rs`](../crates/signal-bot/src/commands/menu_locale.rs) (`!translation-in-chat`) |
