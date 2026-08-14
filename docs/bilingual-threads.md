# Bilingual Threads

Status: **implemented**. Distinct from [Language Threads](language-threads.md) (multilingual main + N sidecars) and [in-chat translation](in-chat-translation.md) (quote-reply in one group).

Exactly **two languages**, exactly **one** sidecar, **no multilingual hub**. Each room has an assigned language and the bot **relays and translates both ways**.

This is **not** N=1 Language Threads. A single multilingual sidecar still posts untranslated into main. Bilingual Threads always translates into the destination room’s assigned language (relay only when the source is already that language).

## Command

Extend the existing Language Threads command. One arg stays Language Threads; two args start Bilingual Threads.

| Command | Effect |
|---------|--------|
| `!translate-me-thread es` | Language Threads (sidecar is `es`; main stays multilingual; sidecar → main relay-only) |
| `!translate-me-thread es en` | **Bilingual Threads**: first lang (`es`) assigned to **main**; second (`en`) assigned to the **one** bridged thread; caller invited |

- Alias `!translation-me-thread` accepts the same two-arg form
- Same two langs (`es es`) refused (same rule as `!translate-me-on`)
- Main group only (not DMs, not from a sidecar)
- Unlike in-chat (`!translate-all-on es en`), **order matters**

## Relay rules

Once locked:

| Direction | Behavior |
|-----------|----------|
| Main → thread | Detect; if already thread lang, relay; else translate to thread lang |
| Thread → main | Detect; if already main lang, relay; else **translate to main lang** |
| Attribution | `{display_name}:\n{body}` |
| Loop safety | One-shot fan-out; never relay the bot (`BotIdentity`) |

No third sidecar. No “post original on main.” Off-pair languages still translate **into** the destination room’s assigned language.

Voice composes: after NEAR Whisper STT, spoken text fans out in-process as the original speaker ([voice-transcription.md](voice-transcription.md)). Do not reintroduce an in-CVM Whisper sidecar.

## Mutual exclusion (three products, one group)

Bilingual Threads cannot run with:

1. **Language Threads** (one-arg / N-lang mode), including an existing N=1 multilingual sidecar
2. **In-chat auto** (`!translate-all-on` / `!translate-me-on`)

Quote `!translate` stays allowed.

**Mode lock**

- First successful **one-arg** subscribe locks **Language Threads**. Later two-arg commands refuse until teardown (`!enable-in-chat`).
- First successful **two-arg** subscribe locks **Bilingual Threads**. No further sidecars. A different pair refuses. One-arg that is not a join helper (below) refuses.
- In-chat active → both thread forms refuse with the switch path (`!enable-threads`); a pending two-arg subscribe applies after teardown.

**Join after bilingual is locked**

- Same pair again (`!translate-me-thread es en`) invites the caller to the existing thread (does not create another group)
- `!translate-me-thread en` (the **thread** lang only) also invites to that sidecar
- `!translate-me-thread es` (main lang) does **not** create a second sidecar; confirms already-in-main / points at the pair
- `!leave` from the sidecar unsubscribes that user. Last member leaving does **not** unlock the pair. Teardown remains `!enable-in-chat` from main

## Persistence

Same encrypted group prefs as Language Threads (`LanguageBridge` on `group-prefs-translation`):

```text
LanguageBridge
  main_lang:        Some(code)  → Bilingual Threads locked
  sidecars:         thread lang → group.… send id  (exactly one)
  sidecar_internal: thread lang → internal_id
  members / member_addresses
```

`main_lang: None` with non-empty sidecars is Language Threads. Legacy prefs without `main_lang` load as Language Threads (`#[serde(default)]`). `DATA_VERSION` stays `1`. In-place Phala upgrade only — do not wipe volumes. See [two-cvm-architecture.md — CVM storage](two-cvm-architecture.md#cvm-storage-keep-intact).

## Key code

| Area | Path |
|------|------|
| Commands + relay | [`crates/signal-bot/src/commands/translate_me.rs`](../crates/signal-bot/src/commands/translate_me.rs) |
| Bridge store (`main_lang`, pending bilingual) | [`crates/signal-bot/src/group_preferences_store.rs`](../crates/signal-bot/src/group_preferences_store.rs) |
| Switch path | [`crates/signal-bot/src/commands/translate_all.rs`](../crates/signal-bot/src/commands/translate_all.rs) (`!enable-threads`) |
| Help copy | [`crates/signal-bot/src/commands/menu_locale.rs`](../crates/signal-bot/src/commands/menu_locale.rs) |

## Smoke checklist

1. Main → `!translate-me-thread es en` → at most one sidecar; main=es, thread=en; caller invited
2. Message in thread appears in main **in Spanish** (translated unless already `es`)
3. Message in main appears in thread **in English** (translated unless already `en`)
4. Second sidecar cannot be added while bilingual is locked
5. Two-arg form refused while Language Threads or in-chat auto is active
6. One-arg Language Threads still works in groups that are not bilingual-locked
7. Voice transcripts fan out on the same bilingual path
