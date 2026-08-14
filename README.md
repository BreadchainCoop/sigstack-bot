# Bread Bot

TEE-hosted Signal bot for **voice transcription** and **group translation**, designed as an interoperable product suite (see [issue #10](https://github.com/BreadchainCoop/sigstack-bot/issues/10)).

Not a general AI chat assistant. Conversation history, tool-calling, and x402 credits have been removed from this fork.

## Products

| Product | Bot role | Where it runs |
|---------|----------|---------------|
| Voice transcription | `BOT__ROLE=translation` | Same Phala CVM / same bot (NEAR AI Whisper; no local sidecar) |
| In-chat (group) translation | `BOT__ROLE=translation` | Same CVM (NEAR AI chat) |
| Language Threads | `BOT__ROLE=translation` | Same CVM |

Add **one** bot to a Signal group. `BOT__ROLE=transcription` is retired.

Details: [docs/two-cvm-architecture.md](docs/two-cvm-architecture.md) · [docs/voice-transcription.md](docs/voice-transcription.md) · [docs/in-chat-translation.md](docs/in-chat-translation.md) · [docs/language-threads.md](docs/language-threads.md)

## Commands

| Product | Command | Effect |
|---------|---------|--------|
| Hub | `!help` / `!info` | Bread Bot hub menus |
| Hub | `!privacy` | Privacy, TEE, and `!verify` (one reply) |
| Hub | `!translation-threads` | Language Threads menu |
| Hub | `!translation-in-chat` | In-chat translation menu |
| Hub | `!help-threads` | How Language Threads works |
| Hub | `!help-in-chat` | How in-chat translation works |
| Hub | `!help-transcription` | How voice transcription works |
| Voice | `!transcription` | Voice product menu |
| Voice | `!transcribe` / `!transcribe-on` / `!transcribe-off` | Manual or auto transcription (default off) |
| Language Threads | `!translate-me-thread <lang>` | Create/join sidecar (main only) |
| Language Threads | `!leave` | Leave this Language Thread (sidecar only) |
| Language Threads | `!commands` | Compact Language Thread command list (sidecar only) |
| Language Threads | `!enable-in-chat` | Tear down Language Threads (switch path to in-chat) |
| In-chat group-wide | `!translate-all-on <lang1> <lang2>` | Auto-translate all messages |
| In-chat group-wide | `!translate-all-off` | Disable group-wide auto |
| In-chat personal | `!translate-me-on <lang1> <lang2>` | Auto-translate this user’s messages only |
| In-chat personal | `!translate-me-off` | Clear this user’s personal auto |
| In-chat | `!enable-threads` | Clear all in-chat auto (switch path to Language Threads) |
| In-chat manual | `!translate <lang>` | Quote-reply translate one message |

Language Threads and in-chat auto are mutually exclusive. Details in the product docs above.

## Architecture

```
Signal group
   └── One Phala CVM (tdx.medium)
         └── signal-api + signal-bot (phone B)
               ├── audio bytes → NEAR AI Whisper Large V3 (GPU TEE)
               └── text → NEAR AI chat
```

- Signal E2E encryption terminates inside this TEE
- No local Whisper sidecar — see [CPU TEE Whisper does not scale](docs/solutions/architecture-patterns/2026-08-13-cpu-tee-whisper-does-not-scale.md)
- After STT, transcripts fan out in-process to in-chat / Language Threads (this number does not receive its own posts)

## Local

```bash
cp docker/translation.env.example docker/translation.env
# Set SIGNAL_PHONE; NEAR_AI_API_KEY (chat + Whisper STT)

docker compose -f docker/compose.translation.yaml --env-file docker/translation.env up -d
```

More thorough local setup (Signal captcha registration, verify SMS/voice codes, and `docker compose logs -f` monitoring): [docs/local-dev/](docs/local-dev/).

## Phala

```bash
# In-place upgrade of the surviving CVM (phone B stays; do not create a replacement)
phala deploy --cvm-id 0e82fa77-8b15-4dbd-89c4-9045ab911353 \
  -c docker/phala.translation.yaml -e docker/phala.translation.env --wait
```

**Do not replace this CVM or wipe its volumes** for a routine upgrade. Disk holds the **registered Signal phone** and **encrypted user prefs**. TEE RAM is cleared on reboot; Phala reattaches named volumes on in-place upgrade. Details: [docs/two-cvm-architecture.md — CVM storage](docs/two-cvm-architecture.md#cvm-storage-keep-intact).

## Project structure

```
crates/
  signal-bot/                   # Binary; live role BOT__ROLE=translation
  signal-bot-transcription/     # Voice / !transcribe* product crate
  whisper-client/               # NEAR Whisper STT client
  near-ai-client/               # NEAR AI (chat + audio transcriptions)
  signal-client/                # Signal CLI REST client
  dstack-client/                # TEE attestation / key derive
  signal-registration-proxy/    # Ops helper for Signal registration
docker/
  compose.translation.yaml      # local one-number stack
  compose.transcription.yaml    # retired stub
  phala.translation.yaml        # prod one-CVM suite
  phala.transcription.yaml      # deprecated stub
docs/
  two-cvm-architecture.md       # one CVM / one phone; CVM storage
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
| `BOT__ROLE` | Live value `translation` (required). `transcription` fail-fasts. |
| `SIGNAL__SERVICE_URL` | Signal CLI REST URL (default `http://signal-api:8080`) |
| `NEAR_AI__API_KEY` | Required (chat + remote Whisper) |
| `WHISPER__ENABLED` / `WHISPER__SERVICE_URL` | Required on the unified bot; URL is NEAR `/v1` |
| `TRANSLATE_ALL__ENABLED` | In-chat `!translate-all-on` / `!translate-me-on` |

See `.env.example` and the docker `*.env.example` files.

## Security

See [.agents/docs/DEVELOPMENT.md](.agents/docs/DEVELOPMENT.md) for the TEE trust model, why Signal CLI must run in the TEE, and attestation (`!verify`).

For coding agents, see [AGENTS.md](AGENTS.md) (includes Compound Engineering install: `/add-plugin compound-engineering`, then `/ce-setup`).

## License

Apache-2.0
