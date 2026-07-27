# Parallel Translation

Status: **MVP implemented** on the translation bot.

One **monolingual** main Signal group plus one **parallel** Signal group. The bot translates both directions so each lane stays in its language.

Distinct from [In-chat translation](in-chat-translation.md) (bilingual main, quote-reply in-thread) and [Language Threads](language-threads.md) (bilingual main + N sidecars).

## Setup

In the main group:

```text
!parallel-on en es
```

- `lang1` = **this** chat (e.g. English)
- `lang2` = parallel group language (e.g. Spanish)
- Bot creates `Parallel Spanish` and adds the organizer

Each other member in the main group:

```text
!parallel-join
```

Leave / stop:

```text
!parallel-leave   # leave the parallel group
!parallel-off     # in main: clear Parallel for the group
```

Menus: `!help` → `!translation` → `!parallel`

## Relay

| Direction | Behavior |
|-----------|----------|
| Main → parallel | Translate to parallel lang (skip NEAR if already that lang) |
| Parallel → main | Translate to main lang |
| Attribution | `{display_name}:\n{body}` |

Bot messages are never relayed (`BotIdentity`).

## Mutual exclusion

Cannot enable Parallel while in-chat auto-translate (`!translate-on`) is active, and vice versa.
