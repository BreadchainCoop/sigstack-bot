# Language Threads

Status: **implemented and verified locally**; Phala TEE redeploy paused (image `daopunk/signal-bot-tee:latest` already pushed for `linux/amd64`).

The sole **cross-group** bridging product on the translation bot: one **multilingual main** Signal chat plus per-language **Language Thread** sidecar groups. Parallel Translation was retired — use this for N=1 or N sidecars with the same rules (no mode switch).

## Problem

In multilingual mutual-aid groups, organizers often dual-post by hand. Monolingual members miss context; bilinguals carry the translation load.

## Solution

| Room | Role |
|------|------|
| **Main group** | Multilingual hub; bot already a member |
| **SigLang {Language} · {disambiguator}** | One Signal sidecar per subscribed language (e.g. `SigLang Spanish · Stacked`) |

Users who want a monolingual lane run `!translate-me-thread <lang>` in **main**. The bot creates or joins the sidecar and invites them. Messages fan out across main and all active threads.

```text
Main (multilingual hub)
  ├── SigLang Spanish · Stacked  ← monolingual ES users
  ├── SigLang English · Stacked  ← monolingual EN users
  └── … (any !list-langs code)
```

Default title is English `SigLang {Language} · {disambiguator}` (main group name when available, else a short hash of the main group id). Members can rename a sidecar with `!rename` from that thread’s `!help`.

N=1 (one sidecar) uses the same relay rules as N=3 — add another language later with no reconfiguration.

## Commands

| Command | Where | Effect |
|---------|--------|--------|
| `!translate-me-thread <lang>` | Main only | Create/join sidecar; invite user |
| `!leave` | Sidecar only | Leave this Language Thread |
| `!disable-threads` | Main | Tear down Language Threads for the group (best-effort remove members); apply pending in-chat enable if any |
| `!rename <name>` | Sidecar only | Change this Language Thread’s group name |
| `!list-langs` | Any | Language codes |
| `!help` / `!privacy` | Any | Hub / privacy menus (`!help` in a sidecar shows the thread menu; `!verify` lives under `!privacy`) |

Menus: `!help` → `!translation-threads`. English-only for now (multi-language UI deferred).

Aliases: `!translation-me-thread es`.

**Also on the translation bot:** [in-chat translation](in-chat-translation.md) (`!translate-all-on` / `!translate-me-on` / quote `!translate`) — same-group only, not a sidecar bridge. **Mutually exclusive with Language Threads** at setup time (refuse + `!disable-in-chat` / `!disable-threads` switch path).

**Not registered:** `!ask`, DM chat, voice/`!transcribe*` (transcription CVM), `!models`.

Menus: `!help` → `!translation-threads` / `!translation-in-chat`. `!in-chat` opens the in-chat menu; `!translation` redirects to both.

## Relay rules (fan-out + BotIdentity)

Loop safety is **one-shot fan-out** from each human inbound (do not rely on main re-processing bot posts) plus **never relay the bot** (`BotIdentity`: phone + learned UUID).

| Direction | Behavior |
|-----------|----------|
| Main → sidecars | Same detected language → **relay**; else **translate** via NEAR AI |
| Sidecar → main | **Relay only** (main stays multilingual; originals kept) |
| Sidecar → other sidecars | **Translate** to each other language (skip source lane; no-op when N=1) |
| Attribution | `{display_name}:\n{body}` (`sourceName` when present) |

Same-language relay skips NEAR. Cross-language calls `near_ai_translate` (configured NEAR model).

Rate limit: one `allow_message(main_id)` per inbound human event (covers fan-out).

**Ops note:** Legacy Parallel Translation Signal groups (if any) are unmanaged after that product’s retirement. Leave them manually and use `!translate-me-thread <lang>` instead.

## Subscribe / unsubscribe flow

1. User in main: `!translate-me-thread es`
2. Resolve language; need invite address (`sourceNumber` preferred, else usable `source`)
3. **First subscriber for that lang:** build English `SigLang …` title/description/welcome → `POST /v1/groups/{bot}` → persist send id + internal id → welcome in sidecar → confirm in main
4. **Later subscribers:** `add_members` on existing sidecar
5. Language switch: remove from old sidecar, add/create new
6. `!leave` (from sidecar): remove from Signal group + store
7. `!disable-threads` (from main): clear bridge, best-effort remove all known members from sidecars; sidecar Signal groups may remain unmanaged
8. Sidecar `!help` → thread menu; `!rename <name>` → `PUT /v1/groups/{bot}/{sendId}`

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

Legacy encrypted prefs that still contain a `parallel_bridge` key are ignored on load and dropped on the next persist.

## Key code

| Area | Path |
|------|------|
| Commands + relay | [`crates/signal-bot/src/commands/translate_me.rs`](../crates/signal-bot/src/commands/translate_me.rs) |
| Bot skip | [`crates/signal-bot/src/bot_identity.rs`](../crates/signal-bot/src/bot_identity.rs) |
| Bridge store | [`crates/signal-bot/src/group_preferences_store.rs`](../crates/signal-bot/src/group_preferences_store.rs) |
| Group REST | [`crates/signal-client/src/client.rs`](../crates/signal-client/src/client.rs) (`create_group`, `add_members`, `remove_members`) |
| Envelope fields | [`crates/signal-client/src/types.rs`](../crates/signal-client/src/types.rs) (`source_name`, `source_number`, …) |
| Help copy | [`crates/signal-bot/src/commands/menu_locale.rs`](../crates/signal-bot/src/commands/menu_locale.rs) |
| Handler registration | [`crates/signal-bot/src/handlers_setup.rs`](../crates/signal-bot/src/handlers_setup.rs) |
| Phase 0 spike notes | [`docs/spikes/2026-07-21-sidecar-groups.md`](spikes/2026-07-21-sidecar-groups.md) |

## Local testing

```bash
cp docker/translation.env.example docker/translation.env
# Set SIGNAL_PHONE (phone B) and NEAR_AI_API_KEY

docker compose -f docker/compose.translation.yaml --env-file docker/translation.env build signal-bot
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env up -d
```

Only **signal-bot** on the translation stack needs rebuild for Language Threads changes.

### Smoke checklist

1. Main group → `!translate-me-thread es` → accept invite → message in main appears in Language Thread (translated or relayed).
2. Add a second lang (`!translate-me-thread en` or `fr`) from main — no reconfiguration; same bridge.
3. Message in a sidecar → appears raw on main + translated in other sidecars; **no echo** back into the source sidecar.
4. Bot-attributed posts are not re-relayed (no ping-pong).
5. From sidecar → `!leave` unsubscribes; from main → `!disable-threads` tears down the product.

Whisper / voice live on the **transcription** stack — see [voice-transcription.md](voice-transcription.md) and [two-cvm-architecture.md](two-cvm-architecture.md).

## Interoperability

- **Transcription** (other CVM) composes with Language Threads or in-chat in the same Signal groups (pairing via `!transcription` on the translation bot).
- **In-chat** translates inside one group thread; **Language Threads** bridges a multilingual main to N monolingual sidecars.

## Phala / TEE (paused)

- Deploy uses **Docker images** in compose, not a public git clone.
- Translation CVM target: **4 GB** (`tdx.medium`) via [`docker/phala.translation.yaml`](../docker/phala.translation.yaml) — **no Whisper** on this box.
- Env template: [`docker/phala.translation.env.example`](../docker/phala.translation.env.example) (secrets; do not commit filled env).
- Fresh CVM ⇒ expect **re-register** Signal phone (volume died with old CVM).

## Trust / privacy notes

- Signal E2E still terminates at Signal CLI inside the TEE (same architecture as before).
- Translation plaintext goes to **NEAR AI** (their GPU TEE / cloud path as configured).
- Operator still sees metadata (timing, sizes, which numbers).
- Sidecar names and bridged posts are visible to members of those Signal groups.

## Open follow-ups

- Resume Phala deploy at 4 GB; confirm memory with `phala cvms get`
- Pin image digest in compose for stronger attestation
- Optional: delete empty sidecars after last member leaves
