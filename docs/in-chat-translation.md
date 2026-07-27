# In-chat (group) translation

Status: **MVP implemented** on the translation bot.

One **bilingual** Signal group (e.g. English + Spanish). The bot detects which side of the pair a message is on and quote-replies with the other language in the **same** main thread.

Distinct from [Language Threads](language-threads.md) (multilingual main + N sidecars). In-chat stays in one Signal group; Language Threads creates sidecar groups.

## Setup

In the group:

```text
!translate-on es en
```

- Stores a bilingual pair for this group (`lang_a` ↔ `lang_b`)
- Order does not matter for detection (either side maps to the other)

Stop:

```text
!translate-off
```

Menus: `!help` → `!translation` → `!in-chat`

## Behavior

| Mode | How | Effect |
|------|-----|--------|
| **Auto** | `!translate-on` active | Every non-command group text message: detect language → if it matches one side of the pair → NEAR translate → quote-reply with `{flag} {translation}` |
| **Manual** | Reply to a message with `!translate <lang>` | Translate only that quoted message |

Not dual-post: the original stays as the human message; the bot only quote-replies the translation.

Skip when language is undetected or not in the pair. Bot messages are never processed (`BotIdentity`). Rate-limited per group (`TRANSLATE_ALL__MAX_MESSAGES_PER_MINUTE`).

Voice notes: the **transcription** bot posts a transcript in-group; with auto-translate on, the **translation** bot then intercepts that text like any other message.

## Key code

| Area | Path |
|------|------|
| Auto on/off + intercept | [`crates/signal-bot/src/commands/translate_all.rs`](../crates/signal-bot/src/commands/translate_all.rs) |
| Quote `!translate` | [`crates/signal-bot/src/commands/translate.rs`](../crates/signal-bot/src/commands/translate.rs) |
| Detect / format helpers | [`crates/signal-bot/src/commands/translate_service.rs`](../crates/signal-bot/src/commands/translate_service.rs) |
| Prefs (`GroupTranslateMode`) | [`crates/signal-bot/src/group_preferences_store.rs`](../crates/signal-bot/src/group_preferences_store.rs) |
| Menus | [`crates/signal-bot/src/commands/menu_locale.rs`](../crates/signal-bot/src/commands/menu_locale.rs) (`!in-chat`) |
