# Voice transcription

Status: **implemented** on the unified Bread Bot (`BOT__ROLE=translation`) on the surviving Phala CVM.

Speech → text via **NEAR AI Whisper Large V3** (GPU TEE). Audio is decrypted in this CVM, stripped of Signal metadata, and uploaded as a generic file. See [one-CVM architecture](two-cvm-architecture.md), [CPU TEE Whisper does not scale](solutions/architecture-patterns/2026-08-13-cpu-tee-whisper-does-not-scale.md), and [issue #8](https://github.com/BreadchainCoop/sigstack-bot/issues/8) under umbrella [#10](https://github.com/BreadchainCoop/sigstack-bot/issues/10).

Users add **one** bot to a group. `!transcription` is the **voice command menu** only (no invite, no pairing). Voice is **default off** (`!transcribe-on` / quote `!transcribe`).

## Where it runs

| Stack | Contents |
|-------|----------|
| Phala (prod) | Same CVM / same bot as translation: `signal-api` (phone B) + `signal-bot` (`BOT__ROLE=translation`, `WHISPER__ENABLED=true`). **No Whisper sidecar.** |
| Local Compose | [`docker/compose.translation.yaml`](../docker/compose.translation.yaml) — one phone + bot; STT is NEAR AI |

`!verify` attests **this** CVM’s compose (one reply). It does not imply Whisper weights live here.

## Signal + in-process fan-out

One Signal number never receives its own group posts. After a successful transcribe (auto or `!transcribe`):

1. Quote-reply `📝 Transcript:` as before.
2. If the group has in-chat auto, run the same intercept path (mode from the **original speaker** / quote author, not the bot).
3. If Language Threads is active, relay the spoken text as if the speaker posted it.

Outbound STT: generic filename (`voice.ogg` / `voice.m4a`). No phone, group id, timestamp, or display name in form fields, headers, or filenames.

**Group invites:** this bot auto-accepts any pending group invite.

## Commands

| Command | Effect |
|---------|--------|
| `!transcription` | Voice product menu (compact command list) |
| `!transcribe-on` / `!transcribe-off` | Toggle auto transcription (DM or group; **default off**) |
| `!transcribe` | Quote a voice note to transcribe it (refuses with a notice if auto is already on) |
| `!help-transcription` | How voice transcription works (NEAR Whisper GPU TEE) |

Hub `!privacy` covers this CVM. `!verify <text>` returns **one** quote.

Auto path: inbound voice notes are transcribed only after `!transcribe-on` (default off).

## Ops

### Local

```bash
cp docker/translation.env.example docker/translation.env
# Set SIGNAL_PHONE; NEAR_AI_API_KEY (chat + Whisper STT)

docker compose -f docker/compose.translation.yaml --env-file docker/translation.env up -d
```

Register the number against this stack’s `signal-api` or proxy `:8081`. Health:

```bash
docker compose -f docker/compose.translation.yaml exec signal-api curl -sf http://localhost:8080/v1/health
```

### Phala

In-place upgrade of the **surviving** translation CVM (do not deploy `phala.transcription.yaml`; do not re-register phone A):

```bash
phala deploy --cvm-id 0e82fa77-8b15-4dbd-89c4-9045ab911353 \
  -c docker/phala.translation.yaml -e docker/phala.translation.env --wait
```

Env template: [`docker/phala.translation.env.example`](../docker/phala.translation.env.example). SKU stays **tdx.medium** (2 vCPU / 4 GB) — remote GPU is the STT speed lever, not a larger TDX. Attestation: `!verify <challenge>` inside Signal.

## Key code

| Area | Path |
|------|------|
| Voice / `!transcribe*` handlers | [`crates/signal-bot-transcription`](../crates/signal-bot-transcription) |
| In-process transcript fan-out | [`crates/signal-bot/src/transcript_fanout.rs`](../crates/signal-bot/src/transcript_fanout.rs) |
| Shared handler trait / errors | [`crates/signal-bot-core`](../crates/signal-bot-core) |
| STT HTTP client | [`crates/whisper-client`](../crates/whisper-client) (NEAR `/audio/transcriptions`) |
| NEAR client (chat + transcribe) | [`crates/near-ai-client`](../crates/near-ai-client) |
| Role wiring | [`crates/signal-bot/src/handlers_setup.rs`](../crates/signal-bot/src/handlers_setup.rs) |
| Compose / Phala | [`docker/compose.translation.yaml`](../docker/compose.translation.yaml), [`docker/phala.translation.yaml`](../docker/phala.translation.yaml) |
