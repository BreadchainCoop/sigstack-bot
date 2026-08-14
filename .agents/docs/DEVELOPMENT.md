# Bread Bot — Development Guide

## Product direction

Interoperable Signal products (see [issue #10](https://github.com/BreadchainCoop/sigstack-bot/issues/10)):

1. **Voice transcription** — `BOT__ROLE=transcription` (NEAR AI Whisper Large V3, not an in-CVM sidecar)
2. **In-chat group translation** — `BOT__ROLE=translation` (see [`docs/in-chat-translation.md`](../../docs/in-chat-translation.md))
3. **Language Threads** — translation role, multilingual main + N sidecars (see [`docs/language-threads.md`](../../docs/language-threads.md))

Architecture: [`docs/two-cvm-architecture.md`](../../docs/two-cvm-architecture.md) (one Phala CVM, two phones). Why STT is remote: [`docs/solutions/architecture-patterns/2026-08-13-cpu-tee-whisper-does-not-scale.md`](../../docs/solutions/architecture-patterns/2026-08-13-cpu-tee-whisper-does-not-scale.md).

Fork legacy removed: general AI chat, tool use (`crates/tools`), x402 payments, in-memory conversation store.

## Security architecture

### TEE trust model

1. **Memory protection**: Code and data in TEE memory encrypted by the CPU (Intel TDX)
2. **Attestation**: Remote parties verify code via TDX quotes (`!verify`)
3. **Isolation**: Hypervisor/host cannot read TEE memory

### Signal CLI must run in the product TEE

Signal E2E encryption terminates at Signal CLI. Each bot process has its own `signal-api` so plaintext only exists in this TEE. Voice bytes leave the TEE only as metadata-stripped audio to NEAR AI Whisper.

### One CVM, two phones, Signal as bus

- Same Phala CVM: transcription bot (phone A, **worker**) + translation bot (phone B, **hub**)
- No `whisper-api` sidecar. Transcription posts audio to NEAR AI Whisper Large V3 (GPU TEE).
- Translation sends **text** to NEAR AI chat. Roles do not HTTP to each other; integration = both bots in the same Signal group.
- Two processes so STT HTTP waits never stall translation polling.

Full hierarchy table: [`docs/two-cvm-architecture.md`](../../docs/two-cvm-architecture.md#bot-hierarchy).

### What attestation proves / does not prove

| Property | Verified by |
|----------|-------------|
| Code in Intel TDX | TDX quote |
| Exact compose | Compose hash |
| Bot / proxy images | Digests pinned in compose |

Does **not** prove Signal CLI image integrity beyond pinning, hide network metadata (timing, sizes, phone numbers), or attest **NEAR AI Whisper weights** (`!verify` is this CVM only).

## `BOT__ROLE`

| Role | Handlers | Requires |
|------|----------|----------|
| `transcription` | Voice, `!transcribe*`, `!transcription` menu, `!help-transcription`, `!verify` (worker — no hub `!help` / `!info` / `!privacy`) | `WHISPER__ENABLED=true` + `NEAR_AI__API_KEY` (remote STT) |
| `translation` | Hub (`!help`, `!info`, `!privacy`, product menus), Language Threads, in-chat, quote `!translate`, `!transcription` pairing, `!verify` | `NEAR_AI__API_KEY` |

Fail-fast if role is missing/invalid or required deps are missing. Do not add a combined role.

## Project structure

```
crates/
  signal-bot/                 # Binary (role-selected handlers)
  signal-bot-core/            # CommandHandler + AppResult
  signal-bot-transcription/   # Voice / !transcribe* product crate
  whisper-client/             # OpenAI-compatible STT client (NEAR Whisper)
  near-ai-client/             # NEAR AI chat + audio transcriptions
  signal-client/
  dstack-client/
  signal-registration-proxy/  # Ops registration helper
docker/
  compose.transcription.yaml  # local phone A (no Whisper sidecar)
  compose.translation.yaml    # local phone B
  phala.translation.yaml      # prod one-CVM suite
  phala.transcription.yaml    # deprecated stub — do not deploy
  Dockerfile / Dockerfile.proxy
docs/
  two-cvm-architecture.md
  voice-transcription.md
  language-threads.md
```

`Dockerfile.whisper` is unused on the live path.

## Local dual Compose

```bash
cp docker/transcription.env.example docker/transcription.env
cp docker/translation.env.example docker/translation.env
# Different SIGNAL_PHONE values; PEER_PHONE on translation = transcription phone;
# NEAR_AI_API_KEY in both env files

docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env up -d
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env up -d
```

Networks: `sigstack-transcription-internal`, `sigstack-translation-internal`.

## Phala deploy

Build `linux/amd64` images (bot + registration proxy only), then **in-place** upgrade the surviving CVM:

```bash
docker buildx build --platform linux/amd64 -t YOUR/signal-bot-tee:latest -f docker/Dockerfile --push .
docker buildx build --platform linux/amd64 -t YOUR/signal-registration-proxy:latest -f docker/Dockerfile.proxy --push .

phala deploy --cvm-id 0e82fa77-8b15-4dbd-89c4-9045ab911353 \
  -c docker/phala.translation.yaml -e docker/phala.translation.env --wait
```

Do **not** `phala deploy -n` against the live CVM. Do **not** deploy `docker/phala.transcription.yaml`. Env template: `docker/phala.translation.env.example`.

Encrypted secrets: `SIGNAL_PHONE` (phone B), `TRANSCRIPTION_PHONE` (phone A), `NEAR_AI_API_KEY` (shared).

Health: Signal CLI `GET /v1/health` on each `signal-api`. Attestation: `!verify <challenge>` (this CVM’s compose, not remote Whisper).

After the first one-CVM merge, re-register phone A on proxy **:8082**. Phone B on **:8081** stays.

### CVM storage — do not wipe

**In-place upgrades only** once phones are registered. TEE RAM wipe is expected; **disk volumes are the product identity.**

| Must keep | Volume | Breakage if lost |
|-----------|--------|------------------|
| Translation Signal session (phone B) | `signal-config-translation` | Hub gone from groups until re-register |
| Transcription Signal session (phone A) | `signal-config-transcription` | Voice worker gone until re-register |
| User prefs (`!translate-me-on`, Language Threads, `!transcribe-on`) | `group-prefs-*` → `/data/group_prefs.enc` | Users must re-enable; suite looks broken |

Use `phala deploy --cvm-id 0e82fa77-8b15-4dbd-89c4-9045ab911353`. Do **not** `phala cvms delete` this CVM, create a replacement, rename those volumes, or `down -v` for an image bump. [`scripts/deploy_phala.sh`](../../scripts/deploy_phala.sh) defaults to that `--cvm-id`.

After upgrade, logs should show `Loaded group preferences for N groups` (not `starting fresh` / `TEE deployment may have changed`), and each `signal-api` should still list its account.

Canonical table: [`docs/two-cvm-architecture.md` — CVM storage](../../docs/two-cvm-architecture.md#cvm-storage-keep-intact). Agent rule: [`AGENTS.md` — CVM storage](../../AGENTS.md#cvm-storage-do-not-wipe).

## Configuration

| Variable | Notes |
|----------|-------|
| `BOT__ROLE` | `transcription` \| `translation` |
| `SIGNAL__SERVICE_URL` | Default `http://signal-api:8080` (transcription container uses `http://signal-api-transcription:8080`) |
| `SIGNAL__PHONE_NUMBER` | Ops phone for this process |
| `SIGNAL__PEER_PHONE` | Peer product bot. Translation: invites transcription. Transcription: must be translation phone for auto-join |
| `NEAR_AI__*` | Both roles (chat on translation; API key also required for remote Whisper) |
| `WHISPER__*` | Transcription role — `SERVICE_URL` is NEAR `/v1`, not `whisper-api:9000` |
| `TRANSLATE_ALL__*` | In-chat translation |
| `GROUP_PREFERENCES__*` | Encrypted group prefs volume |
| `DSTACK__SOCKET_PATH` | `/var/run/dstack.sock` in Phala |

## Testing

```bash
cargo test
cargo build --release
```

Before finishing Rust work: `npm run ci` / `pnpm run ci` (never bare `pnpm ci`).

## Registration proxy

Ops helper on the one CVM: **:8081** phone B (translation), **:8082** phone A (transcription). Multi-tenant “create your personal AI bot” web UX is out of scope; Stripe client site is issue #10 follow-up.

## Website

`web/` is legacy personal-AI framing. Do not treat it as the product suite storefront until the Stripe website work lands.
