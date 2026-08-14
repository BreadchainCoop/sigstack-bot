# Language Threads

Status: **implemented and verified locally**; Phala TEE redeploy paused (image `daopunk/signal-bot-tee:latest` already pushed for `linux/amd64`).

The sole **cross-group** bridging product: one **multilingual main** Signal chat plus per-language **Language Thread** sidecar groups. Parallel Translation was retired — use this for N=1 or N sidecars with the same rules (no mode switch). Voice and hub menus live on the same bot — see [two-cvm-architecture.md](two-cvm-architecture.md).

## Problem

In multilingual mutual-aid groups, organizers often dual-post by hand. Monolingual members miss context; bilinguals carry the translation load.

## Solution

| Room | Role |
|------|------|
| **Main group** | Multilingual hub; bot already a member |
| **{Language} · {disambiguator}** | One Signal sidecar per subscribed language (e.g. `Spanish · Stacked`) |

Users who want a monolingual lane run `!translate-me-thread <lang>` in **main**. The bot creates or joins the sidecar and invites them. Messages fan out across main and all active threads.

```text
Main (multilingual hub)
  ├── Spanish · Stacked  ← monolingual ES users
  ├── English · Stacked  ← monolingual EN users
  └── … (any !list-langs code)
```

Default title is English `{Language} · {disambiguator}` (main group name when available, else a short hash of the main group id). Members can rename a sidecar with `!rename` from that thread’s `!commands` menu.

N=1 (one sidecar) uses the same relay rules as N=3 — add another language later with no reconfiguration.

## Commands

| Command | Where | Effect |
|---------|--------|--------|
| `!translate-me-thread <lang>` | Main only | Create/join sidecar; invite user |
| `!leave` | Sidecar only | Leave this Language Thread |
| `!enable-in-chat` | Main | Tear down Language Threads for the group (best-effort remove members); apply pending in-chat enable if any |
| `!rename <name>` | Sidecar only | Change this Language Thread’s group name |
| `!commands` | Sidecar only | Compact Language Thread command list |
| `!list-langs` | Any | Language codes |
| `!help-threads` | Any | How Language Threads works (use case + flow) |
| `!help` / `!privacy` | Any | Hub menus (`!help` is always the Bread Bot hub; `!privacy` and `!verify` on translation bot) |

Menus: `!help` → `!translation-threads`. English-only for now (multi-language UI deferred).

Aliases: `!translation-me-thread es`.

**Also on the translation bot:** [in-chat translation](in-chat-translation.md) (`!translate-all-on` / `!translate-me-on` / quote `!translate`) — same-group only, not a sidecar bridge. **Mutually exclusive with Language Threads** at setup time (refuse + `!enable-threads` / `!enable-in-chat` switch path).

**Not registered on translation (worker CVM handles these):** `!ask`, DM chat, voice/`!transcribe*`, `!transcription` product menu on the transcription bot, `!models`.

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
3. **First subscriber for that lang:** build English ` …` title/description/welcome → `POST /v1/groups/{bot}` → persist send id + internal id → welcome in sidecar → confirm in main
4. **Later subscribers:** `add_members` on existing sidecar
5. Language switch: remove from old sidecar, add/create new
6. `!leave` (from sidecar): remove from Signal group + store
7. `!enable-in-chat` (from main): notify each Language Thread, remove members from sidecars (best-effort), clear bridge; sidecar Signal groups may remain unmanaged
8. Sidecar `!commands` → thread menu; `!rename <name>` → `PUT /v1/groups/{bot}/{sendId}`

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

The same encrypted file on `group-prefs-translation` also holds in-chat `!translate-me-on` / `!translate-all-on`. An **in-place** Phala CVM upgrade reattaches that volume (and `signal-config-translation`, the hub’s registered Signal phone). Users should not have to re-subscribe. Replacing the CVM or wiping volumes drops bridges, personal auto-translate, **and** the bot’s Signal session — the suite looks broken until ops re-registers and users opt in again. See [two-cvm-architecture.md — CVM storage](two-cvm-architecture.md#cvm-storage-keep-intact).

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
5. From sidecar → `!leave` unsubscribes; from main → `!enable-in-chat` tears down the product.

Whisper / voice run in the same bot process (NEAR AI Whisper) — after STT, spoken text fans out into Language Threads as the original speaker. See [voice-transcription.md](voice-transcription.md) and [two-cvm-architecture.md](two-cvm-architecture.md).

## Interoperability

- **Voice** composes with Language Threads or in-chat in the same Signal groups. Transcripts fan out in-process (this number does not receive its own posts).
- **In-chat** translates inside one group thread; **Language Threads** bridges a multilingual main to N monolingual sidecars.

## Phala / TEE

- One CVM on Phala (`tdx.medium` = 2 vCPU / 4 GB RAM): [`docker/phala.translation.yaml`](../docker/phala.translation.yaml) — one Signal number (phone B). No Whisper sidecar; STT is NEAR AI. See [CPU TEE Whisper does not scale](solutions/architecture-patterns/2026-08-13-cpu-tee-whisper-does-not-scale.md).
- Deploy uses **Docker images** (digest-pinned in env), not a public git clone. Upgrade in place: `phala deploy --cvm-id 0e82fa77-8b15-4dbd-89c4-9045ab911353`.
- Env template: [`docker/phala.translation.env.example`](../docker/phala.translation.env.example) (secrets; do not commit filled env).
- Registration proxy on this CVM: `:8081` phone B.

## Trust / privacy notes

- Signal E2E still terminates at Signal CLI inside the TEE (same architecture as before).
- Translation plaintext and voice audio (metadata-stripped) go to **NEAR AI** (chat + Whisper Large V3 GPU TEE).
- Operator still sees metadata (timing, sizes, which numbers).
- Sidecar names and bridged posts are visible to members of those Signal groups.

## Open follow-ups

- Optional: delete empty sidecars after last member leaves
