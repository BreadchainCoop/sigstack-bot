# Bread Bot

TEE-hosted Signal bots for **voice transcription** and **group translation**, designed as an interoperable product suite (see [issue #10](https://github.com/BreadchainCoop/sigstack-bot/issues/10)).

Not a general AI chat assistant. Conversation history, tool-calling, and x402 credits have been removed from this fork.

## Products

| Product | Bot role | Where it runs |
|---------|----------|---------------|
| Voice transcription | `BOT__ROLE=transcription` | Same Phala CVM as translation (NEAR AI Whisper; no local sidecar) |
| In-chat (group) translation | `BOT__ROLE=translation` | Same CVM (NEAR AI chat) |
| Language Threads | `BOT__ROLE=translation` | Same CVM |

Pair products by adding **both bots** (two phone numbers) to the same Signal group. Locally, two Compose projects mock two phones.

**Bot hierarchy:** the **translation** bot is the Bread Bot **hub** (`!help`, `!info`, `!privacy`, translation products, transcription pairing). The **transcription** bot is a **specialized worker** (voice → text via `!transcription` / `!transcribe*` only). See [docs/two-cvm-architecture.md](docs/two-cvm-architecture.md#bot-hierarchy).

Details: [docs/two-cvm-architecture.md](docs/two-cvm-architecture.md) · [docs/voice-transcription.md](docs/voice-transcription.md) · [docs/in-chat-translation.md](docs/in-chat-translation.md) · [docs/language-threads.md](docs/language-threads.md)

## Translation commands

| Product | Command | Where | Effect |
|---------|---------|-------|--------|
| Hub | `!help` / `!info` | Translation bot only | Bread Bot hub menus |
| Hub | `!privacy` | Translation bot only | Privacy, TEE, and `!verify` (dual quotes in paired groups) |
| Hub | `!translation-threads` | Translation bot | Language Threads menu |
| Hub | `!translation-in-chat` | Translation bot | In-chat translation menu |
| Hub | `!help-threads` | Translation bot | How Language Threads works |
| Hub | `!help-in-chat` | Translation bot | How in-chat translation works |
| Hub | `!help-transcription` | Translation bot | How voice transcription works (guide; worker runs on transcription bot) |
| Transcription | `!transcription` | Transcription bot | Voice product menu |
| Language Threads | `!translate-me-thread <lang>` | Main only | Create/join sidecar |
| Language Threads | `!leave` | Sidecar only | Leave this Language Thread |
| Language Threads | `!commands` | Sidecar only | Compact Language Thread command list |
| Language Threads | `!enable-in-chat` | Main | Tear down Language Threads (switch path to in-chat) |
| In-chat group-wide | `!translate-all-on <lang1> <lang2>` | Group | Auto-translate all messages |
| In-chat group-wide | `!translate-all-off` | Group | Disable group-wide auto |
| In-chat personal | `!translate-me-on <lang1> <lang2>` | Group (not sidecar) | Auto-translate this user’s messages only |
| In-chat personal | `!translate-me-off` | Group (not sidecar) | Clear this user’s personal auto |
| In-chat | `!enable-threads` | Group | Clear all in-chat auto (switch path to Language Threads) |
| In-chat manual | `!translate <lang>` | Group | Quote-reply translate one message |

Language Threads and in-chat auto are mutually exclusive. Details in the product docs above.

## Architecture

```
Signal group
   └── One Phala CVM (tdx.medium)
         ├── signal-api + signal-bot (phone B, translation hub)
         └── signal-api + signal-bot (phone A, transcription worker)
               └── audio bytes → NEAR AI Whisper Large V3 (GPU TEE)
```

- Signal E2E encryption terminates inside this TEE
- No local Whisper sidecar — see [CPU TEE Whisper does not scale](docs/solutions/architecture-patterns/2026-08-13-cpu-tee-whisper-does-not-scale.md)
- Translation uses NEAR AI on text (including transcripts posted by the transcription bot)

## Local dual stack

```bash
cp docker/transcription.env.example docker/transcription.env
cp docker/translation.env.example docker/translation.env
# Two different SIGNAL_PHONE values; NEAR_AI_API_KEY in both env files

docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env up -d
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env up -d
```

More thorough local setup (Signal captcha registration, verify SMS/voice codes, and `docker compose logs -f` monitoring): [docs/local-dev/](docs/local-dev/).

## Phala

```bash
# In-place upgrade of the surviving CVM (phone B stays; do not create a replacement)
phala deploy --cvm-id 0e82fa77-8b15-4dbd-89c4-9045ab911353 \
  -c docker/phala.translation.yaml -e docker/phala.translation.env --wait
```

**Do not replace this CVM or wipe its volumes** for a routine upgrade. Disk holds **registered Signal phones** and **encrypted user prefs**. TEE RAM is cleared on reboot; Phala reattaches named volumes on in-place upgrade. After the first one-CVM merge, re-register phone A on proxy `:8082`. Details: [docs/two-cvm-architecture.md — CVM storage](docs/two-cvm-architecture.md#cvm-storage-keep-intact).

## Project structure

```
crates/
  signal-bot/                   # Binary; role via BOT__ROLE
  whisper-client/               # NEAR Whisper STT client
  near-ai-client/               # NEAR AI (chat + audio transcriptions)
  signal-client/                # Signal CLI REST client
  dstack-client/                # TEE attestation / key derive
  signal-registration-proxy/    # Ops helper for Signal registration
docker/
  compose.transcription.yaml
  compose.translation.yaml
  phala.translation.yaml        # prod one-CVM suite
  phala.transcription.yaml      # deprecated stub
docs/
  two-cvm-architecture.md       # one CVM / two phones; CVM storage
  in-chat-translation.md
  language-threads.md
```

## Build & test

```bash
cargo build --release
cargo test
```

## Commit messages

This repo uses [commitlint](https://github.com/conventional-changelog/commitlint) with `@commitlint/config-conventional`. After clone, run `npm install` once so Husky can install hooks.

- **commit-msg** — rejects non-conventional messages at commit time  
- **pre-push** — rejects a push if any commit being pushed fails lint  

Format: `type(scope): subject` — e.g. `feat: add whisper timeout`, `fix(docker): pin signal-api digest`. Common types: `feat`, `fix`, `docs`, `chore`, `refactor`, `test`, `ci`.

## Configuration

| Variable | Description |
|----------|-------------|
| `BOT__ROLE` | `transcription` or `translation` (required) |
| `SIGNAL__SERVICE_URL` | Signal CLI REST URL (default `http://signal-api:8080`) |
| `NEAR_AI__API_KEY` | Required for both roles (chat + remote Whisper) |
| `WHISPER__ENABLED` / `WHISPER__SERVICE_URL` | Transcription role; URL is NEAR `/v1` |
| `TRANSLATE_ALL__ENABLED` | In-chat `!translate-all-on` / `!translate-me-on` (translation role) |

See `.env.example` and the docker `*.env.example` files.

## Security

See [.agents/docs/DEVELOPMENT.md](.agents/docs/DEVELOPMENT.md) for the TEE trust model, why Signal CLI must run in the TEE, and attestation (`!verify`).

For coding agents, see [AGENTS.md](AGENTS.md) (includes Compound Engineering install: `/add-plugin compound-engineering`, then `/ce-setup`).

## License

Apache-2.0
