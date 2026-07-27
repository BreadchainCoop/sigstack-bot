# Sigstack Bot

TEE-hosted Signal bots for **voice transcription** and **group translation**, designed as an interoperable product suite (see [issue #10](https://github.com/BreadchainCoop/sigstack-bot/issues/10)).

Not a general AI chat assistant. Conversation history, tool-calling, and x402 credits have been removed from this fork.

## Products

| Product | Bot role | Where it runs |
|---------|----------|---------------|
| Voice transcription | `BOT__ROLE=transcription` | Own CVM / Compose stack (with Whisper) |
| In-chat (group) translation | `BOT__ROLE=translation` | Shared translation CVM |
| Language Threads | `BOT__ROLE=translation` | Shared translation CVM |
| Parallel Translation | planned | Shared translation CVM |

Pair products by adding **both bots** (two phone numbers) to the same Signal group. Signal is the bus — there is no Docker network between CVMs.

Details: [docs/two-cvm-architecture.md](docs/two-cvm-architecture.md) · [docs/language-threads.md](docs/language-threads.md)

## Architecture

```
Signal group
   ├── Transcription CVM (4 GB): signal-api + whisper-api + signal-bot
   └── Translation CVM   (4 GB): signal-api + signal-bot (+ registration proxy)
```

- Signal E2E encryption terminates inside each TEE
- Whisper stays on the transcription CVM only (local Docker HTTP)
- Translation uses NEAR AI on text (including transcripts posted by the transcription bot)

## Local dual stack

```bash
cp docker/transcription.env.example docker/transcription.env
cp docker/translation.env.example docker/translation.env
# Two different SIGNAL_PHONE values; NEAR_AI_API_KEY in translation.env

docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env up -d
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env up -d
```

## Phala

```bash
# Build linux/amd64 images, then deploy each compose to its own CVM @ tdx.medium (4 GB)
phala deploy … -c docker/phala.transcription.yaml …
phala deploy … -c docker/phala.translation.yaml …
```

## Project structure

```
crates/
  signal-bot/                   # Binary; role via BOT__ROLE
  whisper-client/               # Whisper HTTP client
  near-ai-client/               # NEAR AI (translation)
  signal-client/                # Signal CLI REST client
  dstack-client/                # TEE attestation / key derive
  signal-registration-proxy/    # Ops helper for Signal registration
docker/
  compose.transcription.yaml
  compose.translation.yaml
  phala.transcription.yaml
  phala.translation.yaml
docs/
  two-cvm-architecture.md
  language-threads.md
```

## Build & test

```bash
cargo build --release
cargo test
```

## Configuration

| Variable | Description |
|----------|-------------|
| `BOT__ROLE` | `transcription` or `translation` (required) |
| `SIGNAL__SERVICE_URL` | Signal CLI REST URL (default `http://signal-api:8080`) |
| `NEAR_AI__API_KEY` | Required for translation role |
| `WHISPER__ENABLED` / `WHISPER__SERVICE_URL` | Required for transcription role |
| `TRANSLATE_ALL__ENABLED` | In-chat `!translate-on` (translation role) |

See `.env.example` and the docker `*.env.example` files.

## Security

See [.agents/DEVELOPMENT.md](.agents/DEVELOPMENT.md) for the TEE trust model, why Signal CLI must run in the TEE, and attestation (`!verify`).

## License

Apache-2.0
