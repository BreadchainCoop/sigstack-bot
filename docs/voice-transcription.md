# Voice transcription

Status: **implemented** as the worker bot (`BOT__ROLE=transcription`) on the **same** Phala CVM as translation.

Speech → text via **NEAR AI Whisper Large V3** (GPU TEE). Audio is decrypted in this CVM, stripped of Signal metadata, and uploaded as a generic file. See [one-CVM architecture](two-cvm-architecture.md), [CPU TEE Whisper does not scale](solutions/architecture-patterns/2026-08-13-cpu-tee-whisper-does-not-scale.md), and [issue #8](https://github.com/BreadchainCoop/sigstack-bot/issues/8) under umbrella [#10](https://github.com/BreadchainCoop/sigstack-bot/issues/10).

This stack is a **specialized worker**, not the Bread Bot hub. Users discover products and pair bots through the **translation** bot (`!help`, `!transcription` invite). This bot only handles voice transcription, its product menu on `!transcription`, and transcription-side TEE attestation. Hierarchy: [two-cvm-architecture.md — Bot hierarchy](two-cvm-architecture.md#bot-hierarchy).

## Where it runs

| Stack | Contents |
|-------|----------|
| Phala (prod) | Same CVM as translation: `signal-api-transcription` (phone A) + `signal-bot` (`BOT__ROLE=transcription`). **No Whisper sidecar.** |
| Local Compose | [`docker/compose.transcription.yaml`](../docker/compose.transcription.yaml) — phone A + bot; STT is NEAR AI |

`!verify` attests **this** CVM’s compose. It does not imply Whisper weights live here.

## Signal as bus

Both bots are members of the same Signal group (two phone numbers).

1. Human sends a voice note.
2. Transcription bot downloads the attachment, posts audio-only multipart to NEAR Whisper, quote-replies with text prefixed by `📝 Transcript:` (configurable via `WHISPER__REPLY_PREFIX`).
3. Translation bot treats that text like any other group message (in-chat auto-translate, Language Threads).

Outbound STT: generic filename (`voice.ogg` / `voice.m4a`). No phone, group id, timestamp, or display name in form fields, headers, or filenames.

## Pairing (translation leads)

1. Set `PEER_PHONE` / `TRANSCRIPTION_PHONE` in translation env to the transcription bot’s E.164 (`SIGNAL__PEER_PHONE`).
2. Set `PEER_PHONE` in transcription env to the **translation** bot’s E.164 (required for auto-join).
3. Translation bot must be a **group admin**.
4. In the group: `!transcription` → translation bot invites the peer if missing and posts the Voice Transcription menu.
5. Transcription bot auto-accepts the invite when the translation peer is already in the group (polls pending invites).

Without `PEER_PHONE` on translation, `!transcription` stubs as unavailable. Without `PEER_PHONE` on transcription, auto-join is disabled.

**Peer trust:** each bot must trust the other’s Signal identity (`TRUSTED_*`, not `UNTRUSTED`). On startup the bot calls `PUT /v1/identities/{self}/trust/{PEER_PHONE}` with `trust_all_known_keys`. If the peer is `UNTRUSTED`, the translation bot will not decrypt transcription posts (so in-chat auto never sees transcripts), and group sends can fail with `Untrusted Identity`.

When the peer is already in the group (or invite pending), the hub stays silent on `!transcription` so the transcription bot can answer with its menu.

**Group invites:** the translation hub auto-accepts any pending group invite. The transcription worker only auto-accepts invites for groups where the translation peer is already a member/admin.

## Commands (transcription bot)

Worker-only — no `!help` / `!info`. Use the translation bot for the Bread Bot hub.

| Command | Effect |
|---------|--------|
| `!transcription` | Product menu (compact command list for this bot) |
| `!transcribe-on` / `!transcribe-off` | Toggle auto transcription (DM or group; **default off**) |
| `!transcribe` | Quote a voice note to transcribe it (refuses with a notice if auto is already on) |
| `!help-transcription` | How voice transcription works (NEAR Whisper GPU TEE; same CVM as the hub) |

Hub `!privacy` (translation bot only) covers both numbers on this CVM. In a paired group, `!verify <text>` returns two quotes (one per Signal number).

Auto path: inbound voice notes are transcribed only after `!transcribe-on` (default off).

## Ops

### Local

```bash
cp docker/transcription.env.example docker/transcription.env
# Set SIGNAL_PHONE (phone A); PEER_PHONE = translation phone (required for auto-join)
# Set NEAR_AI_API_KEY (same key as translation)

docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env up -d
```

Register phone A against this stack’s `signal-api`. Health:

```bash
docker compose -f docker/compose.transcription.yaml exec signal-api curl -sf http://localhost:8080/v1/health
```

### Phala

In-place upgrade of the **surviving** translation CVM (do not deploy `phala.transcription.yaml`):

```bash
phala deploy --cvm-id 0e82fa77-8b15-4dbd-89c4-9045ab911353 \
  -c docker/phala.translation.yaml -e docker/phala.translation.env --wait
```

Env template: [`docker/phala.translation.env.example`](../docker/phala.translation.env.example). SKU stays **tdx.medium** (2 vCPU / 4 GB) — remote GPU is the STT speed lever, not a larger TDX. After the first merge, **re-register phone A** on proxy `:8082` (the old transcription CVM was deleted). Attestation: `!verify <challenge>` inside Signal.

## Key code

| Area | Path |
|------|------|
| Voice / `!transcribe*` handlers | [`crates/signal-bot-transcription`](../crates/signal-bot-transcription) |
| Shared handler trait / errors | [`crates/signal-bot-core`](../crates/signal-bot-core) |
| STT HTTP client | [`crates/whisper-client`](../crates/whisper-client) (NEAR `/audio/transcriptions`) |
| NEAR client (chat + transcribe) | [`crates/near-ai-client`](../crates/near-ai-client) |
| Pairing (`!transcription` on translation) | [`crates/signal-bot/src/commands/product_menus.rs`](../crates/signal-bot/src/commands/product_menus.rs) |
| Role wiring | [`crates/signal-bot/src/handlers_setup.rs`](../crates/signal-bot/src/handlers_setup.rs) |
| Compose / Phala | [`docker/compose.transcription.yaml`](../docker/compose.transcription.yaml), [`docker/phala.translation.yaml`](../docker/phala.translation.yaml) |
