# Voice transcription

Status: **implemented** on its own Phala / Compose stack (`BOT__ROLE=transcription`).

Speech → text inside Signal via Whisper in the same CVM as the transcription bot. See [two-CVM architecture](two-cvm-architecture.md) and [issue #8](https://github.com/BreadchainCoop/sigstack-bot/issues/8) under umbrella [#10](https://github.com/BreadchainCoop/sigstack-bot/issues/10).

## Where it runs

| Stack | Contents |
|-------|----------|
| Transcription CVM / Compose | `signal-api` (phone A) + `whisper-api` + `signal-bot` (`BOT__ROLE=transcription`) |
| Translation CVM | Does **not** run Whisper |

Whisper HTTP (`http://whisper-api:9000`) is **intra**-transcription-stack only. There is no cross-CVM Docker or HTTP link to the translation bot.

## Signal as bus

Both bots are members of the same Signal group (two phone numbers).

1. Human sends a voice note.
2. Transcription bot downloads the attachment, calls Whisper, quote-replies with text prefixed by `📝 Transcript:` (configurable via `WHISPER__REPLY_PREFIX`).
3. Translation bot treats that text like any other group message (in-chat auto-translate, Parallel, Language Threads).

## Pairing (translation leads)

1. Set `PEER_PHONE` in translation env to the transcription bot’s E.164 (`SIGNAL__PEER_PHONE`).
2. Translation bot must be a **group admin**.
3. In the group: `!transcription` → translation bot invites the peer if missing.
4. Accept the Signal invite on the transcription number.
5. Send `!transcription` again → the transcription bot answers with its menu (translation stays silent when paired).

Without `PEER_PHONE`, translation still stubs `!transcription` as unavailable.

## Commands (transcription bot)

| Command | Effect |
|---------|--------|
| `!transcription` | Product menu |
| `!transcribe-on` / `!transcribe-off` | Toggle auto transcription (DM or group) |
| `!transcribe` | Quote a voice note to transcribe it |
| `!help` / `!privacy` / `!verify` | Help, privacy, TEE attestation |

Auto path: inbound voice notes are transcribed when enabled (default on).

## Ops

### Local

```bash
cp docker/transcription.env.example docker/transcription.env
# Set SIGNAL_PHONE (phone A); optional PEER_PHONE = translation phone

docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env up -d
```

Register phone A against this stack’s `signal-api`. Health:

```bash
docker compose -f docker/compose.transcription.yaml exec whisper-api curl -sf http://localhost:9000/health
docker compose -f docker/compose.transcription.yaml exec signal-api curl -sf http://localhost:8080/v1/health
```

### Phala

```bash
# Build & push linux/amd64 images, then:
phala deploy … -c docker/phala.transcription.yaml -e docker/phala.transcription.env --wait -t tdx.medium
```

Env template: [`docker/phala.transcription.env.example`](../docker/phala.transcription.env.example). Target size: **4 GB** (`tdx.medium`). Attestation: `!verify <challenge>` inside Signal.

## Key code

| Area | Path |
|------|------|
| Voice / `!transcribe*` handlers | [`crates/signal-bot-transcription`](../crates/signal-bot-transcription) |
| Shared handler trait / errors | [`crates/signal-bot-core`](../crates/signal-bot-core) |
| Whisper HTTP client | [`crates/whisper-client`](../crates/whisper-client) |
| Pairing (`!transcription` on translation) | [`crates/signal-bot/src/commands/product_menus.rs`](../crates/signal-bot/src/commands/product_menus.rs) |
| Role wiring | [`crates/signal-bot/src/handlers_setup.rs`](../crates/signal-bot/src/handlers_setup.rs) |
| Compose / Phala | [`docker/compose.transcription.yaml`](../docker/compose.transcription.yaml), [`docker/phala.transcription.yaml`](../docker/phala.transcription.yaml) |
