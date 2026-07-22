# Language Threads (mutual-aid sidecars)

Status: **implemented and verified locally**; Phala TEE redeploy paused (image `daopunk/signal-bot-tee:latest` already pushed for `linux/amd64`).

This document describes what shipped for bilingual mutual-aid groups: one main Signal chat plus per-language **Language Thread** sidecar groups, bridged by the bot.

## Problem

In bilingual NYC mutual-aid groups, organizers often dual-post EN+ES by hand. Monolingual members miss context; bilinguals carry the translation load.

## Solution

| Room | Role |
|------|------|
| **Main group** | Bilingual hub; bot already a member |
| **Language Thread {Language}** | One Signal sidecar per subscribed language (e.g. `Language Thread Spanish`) |

Users who want a monolingual lane run `!translate-me-on <lang>` in **main**. The bot creates or joins the sidecar and invites them. Messages are relayed/translated across main and all active threads.

```text
Main (bilingual)
  ├── Language Thread Spanish  ← monolingual ES users
  ├── Language Thread English  ← monolingual EN users
  └── … (any !list-langs code)
```

## Commands (alpha surface)

Registered handlers are slimmed for alpha:

| Command | Where | Effect |
|---------|--------|--------|
| `!translate-me-on <lang>` | Main only | Create/join sidecar; invite user |
| `!translate-me-off` | Main or sidecar | Leave sidecar |
| `!list-langs` | Any | Language codes |
| `!help` / `!privacy` | Any | Menus |
| `!set-en` / `!set-es` | Group | Menu language |
| `!verify` / `!clear` / `!models` | As before | TEE / session |

Aliases: `!translate-me on es`, `!translation-me-on es`, etc.

**Not registered** (code kept in tree): `!ask`, DM chat, `!translate-on`/`off`, quote `!translate`, voice/`!transcribe*`.

## Relay rules

Bot **never** processes its own messages (phone + learned UUID via `BotIdentity`).

| Direction | Behavior |
|-----------|----------|
| Main → sidecar | Same detected language → **relay**; else **translate** via NEAR AI |
| Sidecar → main | **Relay only** (main is bilingual) |
| Sidecar → other sidecars | **Translate** to each other language |
| Attribution | `{display_name}:\n{body}` (`sourceName` when present) |

Same-language relay skips NEAR. Cross-language calls `near_ai_translate` (configured NEAR model).

Rate limit: one `allow_message(main_id)` per inbound human event (covers fan-out).

## Subscribe / unsubscribe flow

1. User in main: `!translate-me-on es`
2. Resolve language; need invite address (`sourceNumber` preferred, else usable `source`)
3. **First subscriber for that lang:** `POST /v1/groups/{bot}` → name `Language Thread Spanish` → persist send id + internal id → welcome in sidecar → confirm in main
4. **Later subscribers:** `add_members` on existing sidecar
5. Language switch: remove from old sidecar, add/create new
6. `!translate-me-off`: remove from Signal group + store

If Signal omits phone number, bot asks the user to DM once, then retry.

## Persistence

Encrypted group prefs (`GroupPreferencesStore`, TEE-derived key when dstack is available):

```text
LanguageBridge (keyed by main group internal_id)
  sidecars:         lang → group.… send id
  sidecar_internal: lang → internal_id (inbound match)
  members:          user key → lang
  member_addresses: user key → invite address
```

In-memory reverse index: sidecar `internal_id` → `(main_id, lang)`.

Local Docker without dstack may not persist prefs across restarts; Phala with dstack does.

## Key code

| Area | Path |
|------|------|
| Commands + relay | [`crates/signal-bot/src/commands/translate_me.rs`](../crates/signal-bot/src/commands/translate_me.rs) |
| Bot skip | [`crates/signal-bot/src/bot_identity.rs`](../crates/signal-bot/src/bot_identity.rs) |
| Bridge store | [`crates/signal-bot/src/group_preferences_store.rs`](../crates/signal-bot/src/group_preferences_store.rs) |
| Group REST | [`crates/signal-client/src/client.rs`](../crates/signal-client/src/client.rs) (`create_group`, `add_members`, `remove_members`) |
| Envelope fields | [`crates/signal-client/src/types.rs`](../crates/signal-client/src/types.rs) (`source_name`, `source_number`, …) |
| Help copy | [`crates/signal-bot/src/commands/menu_locale.rs`](../crates/signal-bot/src/commands/menu_locale.rs) |
| Handler registration | [`crates/signal-bot/src/main.rs`](../crates/signal-bot/src/main.rs) |
| Phase 0 spike notes | [`docs/spikes/2026-07-21-sidecar-groups.md`](spikes/2026-07-21-sidecar-groups.md) |

## Local testing

```bash
cd docker
docker compose build signal-bot && docker compose up -d signal-bot
```

Only **signal-bot** needs rebuild. Smoke: main group → `!translate-me-on es` → accept invite → message in main appears in Language Thread (translated or relayed).

## Phala / TEE (paused)

- Deploy uses **Docker images** in compose, not a public git clone. Pushing `daopunk/signal-bot-tee:latest` is what the CVM pulls.
- Target was a **4 GB** CVM (`tdx.medium`) named `dstack-app-hqvaf`; previous `dstack-app-cxswu` was removed.
- Env: `deploy/dstack-app-cxswu/phala.env` (secrets; do not commit).
- Fresh CVM ⇒ expect **re-register** Signal phone (volume died with old CVM).
- Image already pushed: `daopunk/signal-bot-tee:latest` @ `sha256:09bb7aca…` (linux/amd64).

## Trust / privacy notes

- Signal E2E still terminates at Signal CLI inside the TEE (same architecture as before).
- Translation plaintext goes to **NEAR AI** (their GPU TEE / cloud path as configured).
- Operator still sees metadata (timing, sizes, which numbers).
- Sidecar names and bridged posts are visible to members of those Signal groups.

## Open follow-ups

- Resume Phala deploy at 4 GB; confirm memory with `phala cvms get`
- Pin image digest in compose for stronger attestation
- Capacity: whisper + bot + signal-api may be tight on 4 GB
- Optional: delete empty sidecars after last member leaves
