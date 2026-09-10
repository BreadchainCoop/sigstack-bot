# Sigstack — Development Guide

## Product direction

Interoperable Signal products:

1. **Voice transcription** — same process (NEAR AI Whisper Large V3, not an in-CVM sidecar)
2. **In-chat group translation** — see [`docs/in-chat-translation.md`](../../docs/in-chat-translation.md)
3. **Language Threads** — multilingual main + N sidecars (see [`docs/language-threads.md`](../../docs/language-threads.md))

Architecture: [`docs/one-cvm-architecture.md`](../../docs/one-cvm-architecture.md) (one Phala CVM, one Signal number). Why STT is remote: [`docs/solutions/architecture-patterns/2026-08-13-cpu-tee-whisper-does-not-scale.md`](../../docs/solutions/architecture-patterns/2026-08-13-cpu-tee-whisper-does-not-scale.md).

Fork legacy removed: general AI chat, tool use (`crates/tools`), x402 payments, in-memory conversation store.

## Security architecture

### TEE trust model

1. **Memory protection**: Code and data in TEE memory encrypted by the CPU (Intel TDX)
2. **Attestation**: Remote parties verify code via TDX quotes (`!verify`)
3. **Isolation**: Hypervisor/host cannot read TEE memory

### Signal CLI must run in the product TEE

Signal E2E encryption terminates at Signal CLI. Plaintext only exists in this TEE. Voice bytes leave the TEE only as metadata-stripped audio to NEAR AI Whisper.

### One CVM, one phone

- Same Phala CVM: one `signal-api` + one `signal-bot` (phone B)
- No `whisper-api` sidecar. The bot posts audio to NEAR AI Whisper Large V3 (GPU TEE) and text to NEAR AI chat.
- Per-message `tokio::spawn` so STT HTTP waits never stall other handlers. After STT, in-chat / Language Threads fan out in-process (this number does not receive its own posts).

### What attestation proves / does not prove

| Property | Verified by |
|----------|-------------|
| Code in Intel TDX | TDX quote |
| Exact compose | Compose hash |
| Bot / proxy images | Digests pinned in compose |

Does **not** prove Signal CLI image integrity beyond pinning, hide network metadata (timing, sizes, phone numbers), or attest **NEAR AI Whisper weights** (`!verify` is this CVM only).

## Process

One `signal-bot` binary: hub (`!help`, `!info`, `!privacy`, product menus), Language Threads, Bilingual Threads, in-chat, voice / `!transcribe*`, `!transcription` menu, quote `!translate`, `!verify`. Requires `NEAR_AI__API_KEY` and `WHISPER__ENABLED=true`.

## Project structure

```
crates/
  signal-bot/                 # Binary (unified handlers)
  signal-bot-core/            # CommandHandler + AppResult
  signal-bot-voice/           # Voice / !transcribe* product crate
  whisper-client/             # OpenAI-compatible STT client (NEAR Whisper)
  near-ai-client/             # NEAR AI chat + audio transcriptions
  signal-client/
  dstack-client/
  signal-registration-proxy/  # Ops registration helper
docker/
  compose.yaml                # local one-number stack
  phala.yaml                  # prod one-CVM suite
  .env.example / .phala.env.example
  Dockerfile / Dockerfile.proxy
docs/
  one-cvm-architecture.md
  voice-transcription.md
  language-threads.md
```

## Local Compose

```bash
cp docker/.env.example docker/.env
# SIGNAL_PHONE; NEAR_AI_API_KEY (chat + Whisper STT)

docker compose -f docker/compose.yaml --env-file docker/.env up -d
```

Network: `sigstack-translation-internal`.

## Phala deploy

Build `linux/amd64` images (bot + registration proxy only), then **in-place** upgrade the surviving CVM:

```bash
docker buildx build --platform linux/amd64 -t YOUR/signal-bot-tee:latest -f docker/Dockerfile --push .
docker buildx build --platform linux/amd64 -t YOUR/signal-registration-proxy:latest -f docker/Dockerfile.proxy --push .

phala deploy --cvm-id 0e82fa77-8b15-4dbd-89c4-9045ab911353 \
  -c docker/phala.yaml -e docker/.phala.env --wait
```

Do **not** `phala deploy -n` against the live CVM. Env template: `docker/.phala.env.example`.

Encrypted secrets: `SIGNAL_PHONE` (phone B), `NEAR_AI_API_KEY`.

Health: Signal CLI `GET /v1/health` on `signal-api`. Attestation: `!verify <challenge>` (this CVM’s compose, not remote Whisper; one reply).

Do not re-register phone A. Proxy **:8081** only.

### Signal username → site Message button

The marketing site is static GitHub Pages. It does **not** discover the bot username at runtime. After the bot starts (or after first register / username ensure), capture the **share token** from CVM logs or `!bot-username` and bake it into Pages via a public Actions variable. Manual ops is intentional — no CVM→GitHub sync. The site assembles `https://signal.me/#eu/<token>` (never store a full URL in `.env` — `#` truncates under dotenv).

1. Pull logs (or DM the bot `!bot-username`):
   ```bash
   phala logs --cvm-id 0e82fa77-8b15-4dbd-89c4-9045ab911353
   ```
2. Find `Signal username ready` with `username=` (e.g. `sigstack.57`) and `username_token=…`, **or** take the second line of the `!bot-username` reply (token only — not a URL).
3. Set the repo GitHub Actions **variable** `PUBLIC_SIGNAL_USERNAME_TOKEN` to that token (never the E.164; the assembled link is public by design). Remove any leftover `PUBLIC_SIGNAL_USERNAME_LINK` var.
4. Redeploy Pages (`workflow_dispatch` on **Pages**, or any `site/**` push). The workflow already passes `vars.PUBLIC_SIGNAL_USERNAME_TOKEN` into the site build.
5. Verify: [sigstack `_app/env.js`](https://breadchaincoop.github.io/sigstack-bot/sigstack/_app/env.js) shows a non-empty `PUBLIC_SIGNAL_USERNAME_TOKEN`.

**When to re-do:** required after Signal **re-register** or a forced username re-claim (discriminator / share token can change). Not required on routine in-place CVM image upgrades that keep `signal-config-translation` (session + username usually persist); confirm via logs if unsure.

**Local:** the same log line appears in Docker Compose `signal-bot` logs. For local site testing, set `PUBLIC_SIGNAL_USERNAME_TOKEN` in `site/.env` when running `site/` (`npm run dev` / build) — separate from `docker/.env`. Site var details: [`site/README.md`](../../site/README.md).

### CVM storage — do not wipe

**In-place upgrades only** once the phone is registered. TEE RAM wipe is expected; **disk volumes are the product identity.**

| Must keep | Volume | Breakage if lost |
|-----------|--------|------------------|
| Signal session (phone B) | `signal-config-translation` | Bot gone from groups until re-register |
| User prefs (`!translate-me-on`, Language Threads) | `group-prefs-translation` → `/data/group_prefs.enc` | Users must re-enable; suite looks broken |

Use `phala deploy --cvm-id 0e82fa77-8b15-4dbd-89c4-9045ab911353`. Do **not** `phala cvms delete` this CVM, create a replacement, rename those volumes, or `down -v` for an image bump. [`scripts/deploy_phala.sh`](../../scripts/deploy_phala.sh) defaults to that `--cvm-id`.

After upgrade, logs should show `Loaded group preferences for N groups` (not `starting fresh` / `TEE deployment may have changed`), and `signal-api` should still list its account. If DeriveKey is missing, set `GROUP_PREFERENCES_LEGACY_COMPOSE_HASH` to the previous compose hash so AppInfo-encrypted prefs still decrypt; the bot then re-saves with an app-id-only key.

Canonical table: [`docs/one-cvm-architecture.md` — CVM storage](../../docs/one-cvm-architecture.md#cvm-storage-keep-intact). Agent rule: [`AGENTS.md` — CVM storage](../../AGENTS.md#cvm-storage-do-not-wipe).

## Configuration

| Variable | Notes |
|----------|-------|
| `SIGNAL__SERVICE_URL` | Default `http://signal-api:8080` |
| `SIGNAL__PHONE_NUMBER` | Ops phone for this process |
| `NEAR_AI__*` | Chat + API key for remote Whisper (required) |
| `WHISPER__*` | Required — `SERVICE_URL` is NEAR `/v1`, not `whisper-api:9000` |
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

Ops helper on the one CVM: **:8081** (phone B). Multi-tenant “create your personal AI bot” web UX is out of scope; Stripe checkout for the product storefront is a separate workstream.

## Website

- **`site/`** — product educational storefront (SvelteKit static, GitHub Pages). Live URL after Pages is enabled: https://breadchaincoop.github.io/sigstack-bot/ · see [`site/README.md`](../../site/README.md). Stripe checkout is not live yet.
- **`web/`** — legacy personal-AI framing. Do **not** treat it as the product suite storefront.
