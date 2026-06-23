# Whisper Voice Transcription Integration

**Date:** 2026-06-22  
**Status:** Draft  
**Authors:** —  
**Related:** `CLAUDE.md` (TEE model), `DESIGN.md` (future enhancements), [signal-translate-bot](https://github.com/decentralparknyc/signal-translate-bot) (LibreTranslate sidecar pattern)

---

## Overview

Add **voice note transcription** to Signal Bot TEE: users send Signal voice messages (DM or group); the bot **automatically** transcribes them inside the **Intel TDX CVM** and replies with text. **Translation is explicit:** users send `!translate <lang>` as a **reply** to a specific message (typically a prior transcript); the bot translates that text via **NEAR AI**.

Transcription runs **locally in the compose stack** (Whisper), not via NEAR AI, so audio never leaves the enclave as raw media. Translation sends **text only** to NEAR AI (existing HTTPS path).

This extends the existing text bot without replacing it — text chat, tools, and `!verify` continue to work.

## Goals

1. **Implicit transcription** — any voice note to the bot (DM) or in a group where the bot is a member triggers Whisper; **no `!transcribe` command**
2. Transcribe audio **inside the CVM** using **Whisper** (default: `small` or `base` model)
3. **`!translate <lang>`** — user **quotes/replies** to a specific message; bot translates that message's text
4. **`!translate-all <lang1> <lang2>`** — **group only** (Signal group chats, including minimal 3-member groups: two users + bot); one-time setup per group; auto-translate subsequent messages between the two languages
5. Keep **Signal CLI + bot + Whisper in the same attested compose file** (same privacy model as today)
6. Support **local dev stack** (`docker-compose.yaml`) and **Phala production** (`phala-compose.yaml`)

### Non-Goals (v1)

- Real-time streaming transcription
- Speaker diarization
- Replacing NEAR AI for general chat
- Auto-translate in groups without `!translate-all` (only paired-lang mode when enabled)
- LibreTranslate sidecar (deferred; NEAR AI for translate in v1)
- Video / image attachments
- On-device Whisper on user's phone

## Decisions Made

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Transcription trigger | **Implicit** on voice attachment | No `!transcribe`; voice note = transcribe |
| Group behavior | **All voice notes** in groups where bot is present | Same implicit handler as DM |
| Transcription engine | **Whisper** (whisper.cpp or compatible server) | Runs offline in TEE; no API key; predictable cost |
| Deployment shape | **Sidecar service** in compose (`whisper-api`) | Isolates CPU/RAM spike |
| NEAR AI role | **Translation** for non-English targets and text; **not** for `*→English` voice when Whisper can translate | See [Whisper translation limits](#whisper-translation-limits) |
| Translation UX (`!translate`) | **Quote-reply** to target message | User picks which message and target language |
| Translation UX (`!translate-all`) | **Group-only** (`groupId`); minimal **3-member** groups (user₁ + user₂ + bot) OK; `!translate-all <lang1> <lang2>`; disable via `!translate-off` | Not 1:1 DMs; 2 humans + bot = practical bilingual chat pattern |
| Reply threading | **All bot outputs quote-reply** the message being transcribed/translated | Links translation to original sender (bot is always the reply author) |
| `!translate-all` text replies | **Translation only** (Option A) | Original text already visible; less noise in busy groups |
| `!translate-all` voice replies | **Transcript + translation** in one quote-reply | Serves native readers and non-native understanding |
| Audio / transcript cache | **None** — no TTL | Transcripts posted to chat are the source of truth for `!translate` |
| Language discovery | `!translate-langs` (full catalog), `!translate-langs-common` (top 12) | Separate from `!translate-all` error message |
| Whisper model (PoC) | **`small`** | Balance accuracy vs CVM RAM |
| x402 billing | **Deferred** until feature complete | — |
| `!help` menu | **Update** when voice/translate commands ship | Users discover features via existing `!help` command |
| Translation backend | **Whisper** for speech `*→English`; **NEAR AI** for all other pairs and text | Keeps English voice path local; avoids NEAR cost/latency |
| Audio plaintext boundary | Decrypt in `signal-api`; transcribe in CVM; **no disk persistence** of audio | Matches conversation-store ephemeral model |
| Default Whisper model | `small` (configurable) | Balance of accuracy vs 4–8 GB CVM RAM |
| Handler routing | **`VoiceHandler`**, **`TranslateHandler`**, **`TranslateAllHandler`** | Separate implicit vs explicit vs group-pair modes |

> **Open:** exact Whisper runtime (`whisper.cpp` HTTP server vs `faster-whisper` vs custom Rust binding) — spike in Phase 0.

## Whisper Translation Limits

**Confirmed:** Whisper supports translation **to English only**, not arbitrary language pairs.

| Whisper `task` | Input | Output |
|----------------|-------|--------|
| `transcribe` | Speech in language X | Text in **same language X** |
| `translate` | Speech in any supported language | Text in **English only** |

Sources: [OpenAI Speech-to-text](https://developers.openai.com/api/docs/guides/speech-to-text) ("We only support translation into English at this time"), [Whisper translation guide](https://www.mintlify.com/openai/whisper/guides/translation).

**Implications for this bot:**

| Scenario | Backend |
|----------|---------|
| Voice note → transcript in original language | Whisper `transcribe` |
| Voice note → **English** text (incl. `!translate-all` with `en` in pair) | Whisper `translate` — **no NEAR AI** |
| Voice note → non-English (e.g. Spanish) | Whisper `transcribe` → NEAR AI (or future LibreTranslate) |
| Text `!translate` → English | NEAR AI (quoted text only; no audio to re-process) |
| Text `!translate` → non-English | NEAR AI |
| `!translate-all` text message | Detect language → if in pair, translate to other lang via **NEAR AI**; quote-reply original with **translation only** |

**Model note:** Use a **multilingual** Whisper model (`small`, `medium`, `large`) — not `.en`-only or `turbo` variants for translation ([whisper docs](https://pypi.org/project/openai-whisper/)).

## Current Architecture (Baseline)

Today the bot only processes **text** messages:

- `signal-api` — `bbernhard/signal-cli-rest-api`; decrypts Signal E2E inside container
- `signal-bot` — polls `GET /v1/receive/{number}`; `BotMessage::from_incoming` **ignores non-text** (`data.message` only)
- `crates/signal-client/src/types.rs` — no attachment fields on `DataMessage`
- NEAR AI — text chat + tools only

```77:91:crates/signal-client/src/types.rs
impl BotMessage {
    pub fn from_incoming(msg: &IncomingMessage) -> Option<Self> {
        let data = msg.envelope.data_message.as_ref()?;
        let text = data.message.clone()?;
        // ...
    }
}
```

Voice notes arrive as **attachments** on `dataMessage`; support must be added end-to-end.

## Proposed Architecture

### High-Level Flow

```
User Signal voice note (E2E encrypted)
        ↓
┌─────────── Intel TDX CVM (one compose stack) ───────────┐
│  signal-api          signal-bot           whisper-api    │
│  (decrypt)    →      (orchestrate)   →    (transcribe)   │
│       ↓                    ↓                  ↓            │
│  attachment bytes    HTTP POST /transcribe   whisper.cpp   │
│  (in memory)         (in memory)           (in memory)   │
│       ↓                    ↓                               │
│              reply text via POST /v2/send                  │
└──────────────────────────────────────────────────────────┘
        ↓
User receives transcript on Signal
```

**`!translate` flow** (quote-reply; routing by target lang):

```
User quote-replies: !translate es
        ↓
Resolve quoted message text
        ↓
Resolve text from quoted message body (transcript reply or user text)
        ↓
NEAR AI: "Translate to {lang}: {text}"
        ↓
Quote-reply with: {translated text}
```

**`!translate-all` flow** (group-only; persistent until disabled):

> **Nuance:** Signal treats 1:1 chats and groups differently. `!translate-all` requires a **group** (`groupId` on the envelope). The smallest valid case is often a **3-member group** (user₁, user₂, bot) — effectively a bilingual conversation with the bot added for transcription/translation, but not a true 1:1 DM. Rejected in bot-only DMs (bot + single user).

```
User in group: !translate-all es en   (once)
        ↓
Store GroupTranslateMode { group_id, lang_a: es, lang_b: en }
        ↓
On each subsequent text OR voice message in that group:
  1. Detect source language (Whisper for voice; whatlang/CLD for text)
  2. If lang == es → output en; if lang == en → output es; else ignore
  3. Voice + target en: prefer whisper translate (local)
  4. All other directions: NEAR AI on extracted text
        ↓
Quote-reply original message:
  - Voice: transcript (source lang) + translation (target lang) in one message
  - Text: translation only (source already in thread)
```

Disable: `!translate-off` (clears group mode).

> Spike Phase 0: confirm Signal quote JSON; pick text language detector for `!translate-all`.

### New / Modified Components

| Component | Type | Responsibility |
|-----------|------|----------------|
| `whisper-api` | **New compose service** | Load Whisper model; expose `POST /transcribe` |
| `crates/whisper-client/` | **New crate** | HTTP client; timeout; error types |
| `crates/signal-client/` | Modify | Attachment metadata on `DataMessage`; download attachment API |
| `crates/signal-bot/` | Modify | `VoiceHandler`, `TranslateHandler`, `TranslateAllHandler`, `GroupTranslateStore` |
| `docker/docker-compose.yaml` | Modify | Add `whisper-api` service + env |
| `docker/phala-compose.yaml` | Modify | Pin whisper image digest; bump CVM memory if needed |
| `docker/Dockerfile.whisper` (optional) | New | Build whisper.cpp server for linux/amd64 |

### Compose Stack (target)

```
services:
  signal-api          # unchanged role — must stay in TEE
  signal-bot          # + voice handler, WHISPER__SERVICE_URL
  whisper-api         # NEW — pinned image, internal network only
  signal-registration-proxy  # unchanged
```

**Attestation:** Adding `whisper-api` changes `phala-compose.yaml` → **new compose hash**. Users re-verify with `!verify` after deploy.

## Security & TEE Considerations

### Must stay true (same as CLAUDE.md)

- **Signal CLI and bot in same CVM** — voice decrypt happens in `signal-api` inside enclave
- **No plaintext audio to disk** — stream attachment to whisper service in memory; drop after transcript
- **No external transcription API** — avoids leaking audio to third parties
- **Translation text** to NEAR AI over HTTPS when Whisper cannot handle the pair (non-English targets, text messages)
- **Whisper `translate`** keeps `*→English` voice path entirely inside CVM (no NEAR AI, no extra cost)

### Threat model

| Protected | Not protected |
|-----------|----------------|
| Audio content in TEE memory | Message timing, attachment sizes (network metadata) |
| Transcript in Signal chat (quote-linked) | Operator sees traffic patterns |
| Compose hash proves whisper service included | Trust in Whisper model binary / Docker image |

### Persistence

| Data | Storage |
|------|---------|
| Voice audio | **Ephemeral** — never written to volume; dropped after processing |
| Transcripts / translations | **Signal chat history** — bot quote-replies; no in-bot TTL |
| `GroupTranslateMode` | In-memory per `group_id` (ephemeral; lost on restart) |
| Whisper model weights | Docker image or read-only volume (not user content) |

## Integration Points

### 1. Signal attachment download

Extend `signal-client` to call signal-cli-rest-api attachment endpoints (exact paths to confirm against [bbernhard/signal-cli-rest-api](https://github.com/bbernhard/signal-cli-rest-api) docs):

- Parse `attachments[]` on incoming `dataMessage` (content type `audio/*`)
- Download attachment bytes for the receiving account
- Return `(mime_type, bytes)` to handler

### 2. `VoiceHandler` (implicit — no command)

```rust
// crates/signal-bot/src/commands/voice.rs (sketch)
pub struct VoiceHandler {
    whisper: Arc<WhisperClient>,
    signal: Arc<SignalClient>,
}

// matches: BotMessage with audio attachment (DM or group)
// does NOT match: text-only messages, !commands
// 1. download attachment
// 2. If group has translate-all with `en` and detected speech is non-English:
//      whisper.translate(bytes)  → English text (skip NEAR AI)
//    Else if translate-all active (other direction):
//      whisper.transcribe(bytes) → NEAR AI translate to target lang
//    Else:
//      whisper.transcribe(bytes) → transcript in source language
// 3. quote-reply original voice message with transcript (and translation if translate-all active)
```

Register in `main.rs` **before** `ChatHandler`. **Groups:** same handler — any voice note in a group the bot receives is transcribed (bot must be a group member). All replies use Signal **quote-reply** so the thread links back to the original sender.

### 3. `TranslateHandler` (`!translate <lang>`)

```rust
// crates/signal-bot/src/commands/translate.rs (sketch)
pub struct TranslateHandler {
    near_ai: Arc<NearAiClient>,
    signal: Arc<SignalClient>,
}

// matches: text starts with "!translate" AND message has quote/reply to another message
// parse: !translate es  |  !translate Spanish  |  !translate en
// 1. resolve quoted message text from Signal quote body
// 2. near_ai.chat("Translate to {lang}: {text}")
// 3. quote-reply the quoted message with translation
```

**Requirements:**
- Must be a **reply** to a specific message (Signal quote). If user sends `!translate es` without quoting, reply: `Reply to the message you want translated with: !translate <language>`
- `<lang>` = ISO 639-1 code or common language name (mapped in bot)
- Resolve text from **quoted message body** (user text or prior bot transcript reply)
- All targets via **NEAR AI** on quoted text (no audio cache; user may quote bot's transcript)
- Subject to NEAR AI billing / timeout / credits when NEAR path is used

### 4. `TranslateAllHandler` (`!translate-all <lang1> <lang2>`)

```rust
// crates/signal-bot/src/commands/translate_all.rs (sketch)
pub struct TranslateAllHandler {
    group_modes: Arc<GroupTranslateStore>,
    whisper: Arc<WhisperClient>,
    near_ai: Arc<NearAiClient>,
    signal: Arc<SignalClient>,
}

// Setup command (group only):
//   !translate-all es en
//   !translate-off
// Stores bidirectional pair for group_id until cleared or bot restart

// On every subsequent group message (text or voice), if mode active:
//   detect lang → if matches lang1 or lang2, translate to the other
//   Voice → English: whisper.translate
//   Voice → other lang: whisper.transcribe + NEAR AI
//   Text: whatlang (or similar) + NEAR AI for both directions
```

**Requirements:**
- **Groups only** — must have Signal `groupId` (includes minimal 3-member groups: two humans + bot). Reject true 1:1 DMs: `!translate-all is only available in group chats`
- Exactly **two** languages (ISO 639-1 or names): `!translate-all es en`
- **One active mode per group** — calling again replaces the pair; `!translate-off` disables
- Bare `!translate-all` (no langs): `Please specify languages to translate between. !translate-all en es`
- `!translate-langs` — full supported language catalog; `!translate-langs-common` — top 12 by speakers
- Does not replace `!translate` for one-off quote-reply translation
- Consider rate limiting / max messages per minute to avoid NEAR AI spam in busy groups

### 5. Extended message types

```rust
pub struct BotMessage {
    // existing fields...
    pub attachments: Vec<AttachmentRef>,
    pub quote: Option<QuotedMessage>,  // NEW — for !translate
}

pub struct QuotedMessage {
    pub author: String,
    pub text: Option<String>,
    pub timestamp: Option<i64>,
    pub attachments: Vec<AttachmentRef>,
}

pub struct AttachmentRef {
    pub id: String,
    pub content_type: String,
    pub filename: Option<String>,
}
```

### 6. Whisper sidecar API (proposed)

| Method | Path | Body | Response |
|--------|------|------|----------|
| `GET` | `/health` | — | `{ "status": "ok", "model": "small" }` |
| `POST` | `/transcribe` | `{ "audio_base64", "mime", "language": "auto" }` | `{ "text", "language", "task": "transcribe" }` |
| `POST` | `/translate` | `{ "audio_base64", "mime", "language": "auto" }` | `{ "text", "language": "en", "task": "translate" }` |

`/translate` maps to Whisper `task=translate` (speech → **English only**). Use multilingual model (`small`+), not `turbo` or `.en` variants.

Internal only — **do not expose** whisper port on Phala public URL (same as `signal-api`).

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `WHISPER__ENABLED` | `true` | Master switch |
| `WHISPER__SERVICE_URL` | `http://whisper-api:9000` | Sidecar base URL |
| `WHISPER__MODEL` | `small` | Model name loaded in sidecar |
| `WHISPER__LANGUAGE` | `auto` | Force language or auto-detect |
| `WHISPER__TIMEOUT` | `120s` | Max transcribe time per voice note |
| `WHISPER__MAX_DURATION_SECS` | `300` | Reject longer clips with user message |
| `WHISPER__REPLY_PREFIX` | `📝 Transcript:` | Signal reply formatting (when not in translate-all-only mode) |
| `TRANSLATE_ALL__ENABLED` | `true` | Master switch for group auto-translate |
| `TRANSLATE_ALL__MAX_MESSAGES_PER_MINUTE` | `30` | Rate limit per group (NEAR AI protection) |

Compose (`docker/.env`):

```bash
WHISPER_ENABLED=true
WHISPER_MODEL=small
```

Mapped to `WHISPER__*` in `signal-bot` service environment.

## User-Facing Behavior

### Transcription (implicit — no command)

All bot outputs **quote-reply** the original message (DM or group).

| Context | Input | Output (quote-reply to original) |
|---------|-------|----------------------------------|
| **DM** | Voice note to bot | `📝 Transcript:\n{text}` |
| **Group** | Any voice note in group (bot is member) | `📝 Transcript:\n{text}` |
| Any | Voice note too long | Quote-reply: `Voice note too long (max 5 min). Send a shorter clip.` |
| Any | Whisper error | Quote-reply: `Could not transcribe voice note. Try again later.` |
| **Group** + `!translate-all es en` active | Spanish voice note | `📝 (es) {transcript}\n🇺🇸 (en) {English}` — Whisper translate for English leg |
| **Group** + `!translate-all es en` active | English voice note | `📝 (en) {transcript}\n🇪🇸 (es) {Spanish}` — transcribe + NEAR AI |
| **Group** + `!translate-all es en` active | Spanish text | `🇺🇸 {English}` only (translation; original visible above) |
| Any | Text message (not a command) | Chat handler, unless `!translate-all` intercepts in group |

There is **no `!transcribe` command**. Sending a voice note *is* the request.

### Translation (`!translate` — quote-reply)

| Input | Output |
|-------|--------|
| Quote-reply: `!translate es` | `🇪🇸 {translated}` via **NEAR AI** |
| Quote-reply: `!translate en` on bot transcript or user text | `🇺🇸 {English}` via **NEAR AI** |
| `!translate es` without quote | Error: reply to a message first |

Works on **any quoted text** — not only bot transcripts.

### Group auto-translate (`!translate-all`)

**Scope:** Any Signal **group chat** where the bot is a member — from large channels down to a **3-person group** (user₁ + user₂ + bot). That minimal group is the intended pattern for two people who want continuous bidirectional translation: it behaves like a shared DM with the bot in the middle, but Signal delivers it as group messages. **Not** supported in a true 1:1 DM (only the bot and one user).

| Input | Output |
|-------|--------|
| `!translate-all es en` (in group) | `Group translate enabled: español ↔ English` |
| `!translate-off` | `Group translate disabled` |
| 1:1 DM (bot + one user) | `!translate-all is only available in group chats` |
| Bare `!translate-all` | `Please specify languages to translate between. !translate-all en es` |
| `!translate-langs` | Full supported language catalog |
| `!translate-langs-common` | Top 12 languages by speakers (e.g. en, zh, hi, es, fr, ar, bn, pt, ru, ja, de, ko) |
| Subsequent messages in group | Quote-reply original with translation when lang matches pair |

**Example thread** (`!translate-all es en` in a 3-member group: María, John, bot):

```
María:  [voice note]
Bot:    ↳ 📝 (es) Hola a todos...
        🇺🇸 (en) Hello everyone...

María:  ¿Alguien va al meetup?
Bot:    ↳ 🇺🇸 Is anyone going to the meetup?
```

### `!help` menu

All new commands must appear in `crates/signal-bot/src/commands/help.rs` when the feature ships. Proposed additions:

```
**Voice & translation:**
- Send a voice note — auto-transcribed (no command needed)
- !translate <lang> — Quote-reply a message to translate it
- !translate-all <lang1> <lang2> — Group only: auto-translate between two languages
- !translate-off — Disable group auto-translate
- !translate-langs — List all supported languages
- !translate-langs-common — List top 12 languages by speakers
```

Optional short note under **Privacy** or a **Voice** subsection: transcription runs locally in the TEE (Whisper); translation uses NEAR AI on text only.

Update `!help` in the same PR/phase as the commands it documents (not deferred).

## Dependencies

### New Rust crate: `whisper-client`

```toml
[dependencies]
reqwest = { workspace = true }
serde = { workspace = true }
thiserror = { workspace = true }
base64 = { workspace = true }
```

### Whisper sidecar (container)

- **Candidate:** [ggml-org/whisper.cpp](https://github.com/ggerganov/whisper.cpp) server mode, or community HTTP wrapper
- **Platform:** `linux/amd64` only for Phala (same constraint as other images)
- **Model files:** baked into image or mounted read-only volume

## Resource Requirements (Phala CVM)

| Model | RAM (approx) | Notes |
|-------|--------------|-------|
| `tiny` | ~1 GB | Fast, lower accuracy |
| `base` | ~1.5 GB | Dev / low-cost |
| `small` | ~2 GB | **Recommended default** |
| `medium` | ~5 GB | May require 8 GB+ CVM |

Current `phala-compose.yaml` deploy suggestion uses **4096 MB** — sufficient for `small` + existing stack + NEAR AI chat/translate.

| Resource | Local dev | Production CVM |
|----------|-----------|----------------|
| vCPU | 2 | 2–4 (transcription is CPU-bound) |
| Memory | 4 GB | 4–8 GB |
| Disk | +1–2 GB (model in image) | 20 GB (existing) |

## Files to Modify

| File | Changes |
|------|---------|
| `Cargo.toml` | Add `crates/whisper-client` to workspace |
| `crates/whisper-client/` | **New** — HTTP client |
| `crates/signal-client/src/types.rs` | Attachments on `DataMessage`; `BotMessage` voice detection |
| `crates/signal-client/src/client.rs` | `download_attachment()` |
| `crates/signal-bot/src/config.rs` | `WhisperConfig` |
| `crates/signal-bot/src/commands/voice.rs` | **New** — implicit voice handler |
| `crates/signal-bot/src/commands/translate.rs` | **New** — `!translate` quote-reply handler |
| `crates/signal-bot/src/commands/translate_all.rs` | **New** — `!translate-all` group mode |
| `crates/signal-bot/src/group_translate_store.rs` | **New** — per-group lang pair state |
| `crates/signal-bot/src/commands/translate_langs.rs` | **New** — `!translate-langs`, `!translate-langs-common` |
| `crates/signal-bot/src/commands/help.rs` | **Modify** — list voice + translate commands in `!help` output |
| `crates/signal-bot/src/commands/mod.rs` | Export handler |
| `crates/signal-bot/src/main.rs` | Register handler; health check whisper |
| `docker/docker-compose.yaml` | Add `whisper-api` service |
| `docker/phala-compose.yaml` | Add `whisper-api`; pin digest; bump memory if needed |
| `docker/Dockerfile.whisper` | **New** (if custom image) |
| `.env.example` / `docker/.env` | Whisper env vars |
| `CLAUDE.md` | Document voice path + verification notes |

## Testing Plan

- [ ] Unit: `BotMessage` parses voice attachment JSON fixtures
- [ ] Unit: `whisper-client` against mock HTTP server
- [ ] Integration: download sample `.ogg` → transcribe → non-empty text
- [ ] Local: DM voice note → transcript
- [ ] Local: group voice note → transcript in group
- [ ] Local: quote-reply `!translate es` on bot transcript → NEAR AI translation
- [ ] Local: `!translate-all es en` → Spanish voice → English via Whisper
- [ ] Local: `!translate-all es en` → English text → Spanish via NEAR AI
- [ ] Local: `!translate` without quote → helpful error
- [ ] Local: `!help` lists voice note behavior and all translate commands
- [ ] Regression: text messages and `!verify` unchanged
- [ ] Phala: `!verify` after compose update; compose hash includes `whisper-api`
- [ ] Load: 60s voice note completes within `WHISPER__TIMEOUT`

## Implementation Phases

### Phase 0: Spike (1–2 days)

- [ ] Confirm signal-cli-rest-api JSON shape for voice attachments on `/v1/receive`
- [ ] Confirm attachment download endpoint and auth requirements
- [ ] Benchmark `whisper.cpp` vs `faster-whisper` in Docker on `linux/amd64`
- [ ] Pick sidecar image/build approach

### Phase 1: Signal attachment pipeline

- [ ] Extend `signal-client` types + download
- [ ] Extend `BotMessage` / receiver to yield voice messages
- [ ] Manual test: log attachment received (no Whisper yet)

### Phase 2: Whisper sidecar + client

- [ ] Add `whisper-api` to local compose
- [ ] Implement `crates/whisper-client`
- [ ] Health check from `signal-bot` startup

### Phase 3: Voice handler + UX

- [ ] Implement `VoiceHandler` (implicit; DM + group) with progress message (`🎤 Transcribing...`)
- [ ] Quote-reply API on `signal-client` send path
- [ ] Error handling, max duration, timeouts
- [ ] End-to-end local Signal test (DM)

### Phase 4: `!translate` (quote-reply)

- [ ] Parse Signal quote metadata on `BotMessage`
- [ ] Implement `TranslateHandler` (NEAR AI on quoted text)
- [ ] Implement `TranslateLangsHandler` (`!translate-langs`, `!translate-langs-common`)
- [ ] Group + DM translate tests

### Phase 5: `!translate-all` (group auto-translate)

- [ ] `GroupTranslateStore` (in-memory per group)
- [ ] `TranslateAllHandler` setup/teardown commands
- [ ] Intercept group text + voice messages when mode active
- [ ] Text language detection (`whatlang` or similar)
- [ ] Rate limiting for NEAR AI in busy groups
- [ ] Update `!help` with full voice/translate command list

### Phase 6: Production hardening

- [ ] Pin whisper image digest in `phala-compose.yaml`
- [ ] Document CVM sizing in `CLAUDE.md`
- [ ] Update attestation / verification docs for new service

### Phase 7 (optional): LibreTranslate sidecar

- [ ] Self-hosted translation in TEE (no NEAR AI for translate)
- [ ] Operator toggle: `TRANSLATE__BACKEND=near_ai|libretranslate`

## Open Questions

1. **Exact attachment API** — document signal-cli-rest-api paths with real voice-note capture (Phase 0)
2. **Quote/reply JSON + send API** — confirm fields for inbound quotes and outbound quote-replies (Phase 0)
3. **Whisper runtime** — whisper.cpp server vs faster-whisper (Python, heavier image) (Phase 0)
4. **Model updates** — how to bump whisper model without breaking compose attestation expectations

## References

- [bbernhard/signal-cli-rest-api](https://github.com/bbernhard/signal-cli-rest-api)
- [OpenAI Speech-to-text](https://developers.openai.com/api/docs/guides/speech-to-text) — translation to English only
- [Whisper translation guide](https://www.mintlify.com/openai/whisper/guides/translation) — `task=translate` behavior
- [signal-translate-bot](https://github.com/decentralparknyc/signal-translate-bot) — sidecar translation pattern
- `docs/plans/base-plan-template.md` — plan template for this folder
- `docs/plans/2024-12-15-tool-use-system-design.md` — design doc pattern
- `docs/plans/x402-payment-integration.md` — optional billing integration later
