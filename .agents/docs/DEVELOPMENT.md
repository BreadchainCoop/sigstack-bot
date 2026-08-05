# Bread Bot — Development Guide

## Product direction

Interoperable Signal products (see [issue #10](https://github.com/BreadchainCoop/sigstack-bot/issues/10)):

1. **Voice transcription** — own CVM (`BOT__ROLE=transcription`)
2. **In-chat group translation** — translation CVM (see [`docs/in-chat-translation.md`](../../docs/in-chat-translation.md))
3. **Language Threads** — translation CVM, multilingual main + N sidecars (see [`docs/language-threads.md`](../../docs/language-threads.md))

Architecture overview: [`docs/two-cvm-architecture.md`](../../docs/two-cvm-architecture.md).

Fork legacy removed: general AI chat, tool use (`crates/tools`), x402 payments, in-memory conversation store.

## Security architecture

### TEE trust model

1. **Memory protection**: Code and data in TEE memory encrypted by the CPU (Intel TDX)
2. **Attestation**: Remote parties verify code via TDX quotes (`!verify`)
3. **Isolation**: Hypervisor/host cannot read TEE memory

### Signal CLI must run in each product TEE

Signal E2E encryption terminates at Signal CLI. Each product CVM runs its own `signal-api` + `signal-bot` so plaintext only exists in that TEE.

### Two CVMs, Signal as bus

- Transcription CVM: Whisper + transcription bot (phone A)
- Translation CVM: translation bot (phone B) + NEAR AI for text
- No cross-CVM Docker network. Integration = both bots in the same Signal group.
- Whisper HTTP (`http://whisper-api:9000`) is **intra**-transcription-stack only.

### What attestation proves / does not prove

| Property | Verified by |
|----------|-------------|
| Code in Intel TDX | TDX quote |
| Exact compose | Compose hash |
| Whisper / bot images | Digests pinned in compose |

Does **not** prove Signal CLI image integrity beyond pinning, or hide network metadata (timing, sizes, phone numbers).

## `BOT__ROLE`

| Role | Handlers | Requires |
|------|----------|----------|
| `transcription` | Voice, `!transcribe*`, help, privacy, verify | Whisper sidecar |
| `translation` | Language Threads (`!translate-me-thread`), in-chat (`!translate-all-on` / `!translate-me-on`), quote `!translate`, menus, verify | `NEAR_AI__API_KEY` |

Fail-fast if role is missing/invalid or required deps are missing.

## Project structure

```
crates/
  signal-bot/                 # Binary (role-selected handlers)
  signal-bot-core/            # CommandHandler + AppResult
  signal-bot-transcription/   # Voice / !transcribe* product crate
  whisper-client/
  near-ai-client/
  signal-client/
  dstack-client/
  signal-registration-proxy/  # Ops registration helper
docker/
  compose.transcription.yaml
  compose.translation.yaml
  phala.transcription.yaml
  phala.translation.yaml
  Dockerfile / Dockerfile.whisper / Dockerfile.proxy
docs/
  two-cvm-architecture.md
  voice-transcription.md
  language-threads.md
```

## Local dual Compose

```bash
cp docker/transcription.env.example docker/transcription.env
cp docker/translation.env.example docker/translation.env
# Different SIGNAL_PHONE values; PEER_PHONE on translation = transcription phone;
# NEAR_AI_API_KEY in translation.env

docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env up -d
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env up -d
```

Networks: `sigstack-transcription-internal`, `sigstack-translation-internal`.

## Phala deploy

Build `linux/amd64` images, then deploy **two** CVMs @ ~4 GB (`tdx.medium`):

```bash
docker buildx build --platform linux/amd64 -t YOUR/signal-bot-tee:latest -f docker/Dockerfile --push .
docker buildx build --platform linux/amd64 -t YOUR/signal-whisper-api:latest -f docker/Dockerfile.whisper --push .
docker buildx build --platform linux/amd64 -t YOUR/signal-registration-proxy:latest -f docker/Dockerfile.proxy --push .

phala deploy … -c docker/phala.transcription.yaml -e docker/phala.transcription.env --wait -t tdx.medium
phala deploy … -c docker/phala.translation.yaml -e docker/phala.translation.env --wait -t tdx.medium
```

Env templates: `docker/phala.transcription.env.example`, `docker/phala.translation.env.example`.

Encrypted secrets: phone numbers per CVM; `PEER_PHONE` for pairing; `NEAR_AI_API_KEY` on translation only.

Health (transcription): Whisper `GET /health` on `:9000`, Signal CLI `GET /v1/health` on `:8080`. Attestation: `!verify <challenge>`.

## Configuration

| Variable | Notes |
|----------|-------|
| `BOT__ROLE` | `transcription` \| `translation` |
| `SIGNAL__SERVICE_URL` | Default `http://signal-api:8080` |
| `SIGNAL__PHONE_NUMBER` | Ops phone for this CVM |
| `SIGNAL__PEER_PHONE` | Peer product bot (translation invites transcription) |
| `NEAR_AI__*` | Translation role |
| `WHISPER__*` | Transcription role |
| `TRANSLATE_ALL__*` | In-chat translation |
| `GROUP_PREFERENCES__*` | Encrypted group prefs volume |
| `DSTACK__SOCKET_PATH` | `/var/run/dstack.sock` in Phala |

## Testing

```bash
cargo test
cargo build --release
```

## Registration proxy

Still useful as an **ops** helper on the translation stack (port 8081) to register phone B. Register phone A against the transcription stack’s `signal-api` via `docker compose exec` / curl. Multi-tenant “create your personal AI bot” web UX is out of scope; Stripe client site is issue #10 follow-up.

## Website

`web/` is legacy personal-AI framing. Do not treat it as the product suite storefront until the Stripe website work lands.
