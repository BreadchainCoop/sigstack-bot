# Local development

Thorough guide for running the dual Compose stacks on your machine, registering Signal phone numbers (including captcha), and tailing logs.

Quick start (env copy + `up -d`) stays in [README.md](../README.md#local-dual-stack). Architecture context: [two-cvm-architecture.md](../two-cvm-architecture.md). TEE / role details: [`.agents/docs/DEVELOPMENT.md`](../../.agents/docs/DEVELOPMENT.md).

Each stack has its own `signal-api`, network, and Signal CLI data volume. Follow **Transcription stack** and/or **Translation stack** end-to-end; use [Using both together](#using-both-together) when you need pairing in one Signal group.

**Already running and code changed?** Plain `up -d` / `restart` keep the old binary — see [Code changes (rebuild the bot)](#code-changes-rebuild-the-bot).

## Prerequisites

- Docker with Compose v2
- Two different Signal-capable phone numbers (E.164), one per stack
- `NEAR_AI_API_KEY` in `docker/translation.env` for the translation bot
- Optional: a local `/var/run/dstack.sock` if you care about attestation paths; local Compose mounts it read-only — registration and day-to-day bot traffic do not require a live Phala socket

## Captcha token

Signal almost always requires a captcha before SMS/voice verification. Both stacks use the same flow:

1. Open [Signal registration captcha](https://signalcaptchas.org/registration/generate.html) in a browser.
2. Solve the challenge.
3. The page redirects to a `signalcaptcha://…` URL. Copy the **entire** token string (starts with `signalcaptcha://`).
   - If the browser does not show the token clearly, open DevTools → Network/Console, or right-click the failed-navigation link and copy the URL.
4. Use that string as the `captcha` field in the register request. Do not strip the `signalcaptcha://` prefix.

Tokens expire quickly — generate a fresh one if registration fails with a captcha error.

---

## Code changes (rebuild the bot)

Local Compose **builds** `signal-bot` from this repo’s `docker/Dockerfile`. It does **not** pull a published bot image.

That means:

| Command | Picks up new Rust / menu code? |
|---------|--------------------------------|
| `up -d` (no `--build`) | **No** — reuses the image already on your machine |
| `restart signal-bot` | **No** — same container, same binary |
| `up -d --build --force-recreate signal-bot` | **Yes** |

After you pull or edit bot code, rebuild and recreate the bot container (Signal registration volumes are untouched):

```bash
# Transcription stack
docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env \
  up -d --build --force-recreate signal-bot

# Translation stack
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env \
  up -d --build --force-recreate signal-bot
```

Confirm the container is new (Created time should be “seconds/minutes ago”):

```bash
docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env \
  ps signal-bot
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env \
  ps signal-bot
```

Then tail logs and look for a fresh `Starting sigstack Signal bot` / `Listening for messages...` line:

```bash
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env \
  logs -f --tail=50 signal-bot
```

Still on old behavior after that? Force a no-cache image rebuild, then recreate:

```bash
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env \
  build --no-cache signal-bot
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env \
  up -d --force-recreate signal-bot
```

(Same pattern with `compose.transcription.yaml` / `transcription.env` for the transcription bot.)

Do **not** use `down -v` to “force a refresh” — that wipes Signal CLI state and you must re-register the phone.

---

## Transcription stack

Compose file: `docker/compose.transcription.yaml`  
Env file: `docker/transcription.env`  
Role: voice transcription (`BOT__ROLE=transcription`). Includes Whisper.

### Env + start

```bash
cp docker/transcription.env.example docker/transcription.env
# Edit: SIGNAL_PHONE = phone A (transcription bot)
# Optional: PEER_PHONE = phone B when pairing with translation

docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env up -d
```

First `up -d` builds `signal-bot` if needed. After later code changes, use [Code changes (rebuild the bot)](#code-changes-rebuild-the-bot) — plain `up -d` keeps the old binary.

Confirm network:

```bash
docker network ls | grep sigstack-transcription
# Expect: sigstack-transcription-internal
```

### Health

`signal-api` `/v1/health` returns **HTTP 204** with an empty body — no printed output and exit code 0 means healthy. Failure exits non-zero (`curl -sf`).

```bash
docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env \
  exec whisper-api curl -sf http://localhost:9000/health

docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env \
  exec signal-api curl -sf http://localhost:8080/v1/health
```

### Register phone A

`signal-api` is not published on the host; call it from inside the container. The number must match `SIGNAL_PHONE` in `docker/transcription.env`.

Replace `+1XXXXXXXXXX` with phone A.

Check whether the number is already registered (skip captcha/register if it appears in the list):

```bash
docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env \
  exec signal-api curl -sS 'http://localhost:8080/v1/accounts'
# Expect JSON array, e.g. ["+1XXXXXXXXXX"]. Empty [] means not registered yet.
```

If not listed, generate a [captcha token](#captcha-token) and register:

```bash
# Start registration (SMS by default)
docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env \
  exec signal-api curl -sS -X POST \
  -H 'Content-Type: application/json' \
  -d '{"captcha":"signalcaptcha://PASTE_TOKEN_HERE","use_voice":false}' \
  'http://localhost:8080/v1/register/+1XXXXXXXXXX'
```

Use `"use_voice":true` if you prefer a voice call for the code.

When the SMS/voice code arrives:

```bash
docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env \
  exec signal-api curl -sS -X POST \
  -H 'Content-Type: application/json' \
  -d '{}' \
  'http://localhost:8080/v1/register/+1XXXXXXXXXX/verify/123456'
```

Confirm the account is present (same accounts call as above):

```bash
docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env \
  exec signal-api curl -sS 'http://localhost:8080/v1/accounts'
```

Restart the bot so it picks up the registered session:

```bash
docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env \
  restart signal-bot
```

### Logs

Idle bots are quiet at `info` — empty polls do not print. Incoming receive lines are mostly `debug`; successful command/handler work logs at `info`.

```bash
docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env \
  logs -f signal-bot
```

Useful variants:

```bash
# All services on this stack
docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env logs -f

# Last 100 lines, then follow
docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env \
  logs -f --tail=100 signal-bot

# signal-api (registration / receive issues)
docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env \
  logs -f signal-api
```

Raise verbosity: set `LOG_LEVEL=debug` in `docker/transcription.env`, then recreate the bot:

```bash
docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env \
  up -d signal-bot
```

### Stop / rebuild

Stop the stack (keeps Signal CLI volumes):

```bash
docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env down
```

After code changes, rebuild/recreate — see [Code changes (rebuild the bot)](#code-changes-rebuild-the-bot). Do **not** use `down -v` unless you intend to wipe Signal CLI state (you will need to re-register the phone).

---

## Translation stack

Compose file: `docker/compose.translation.yaml`  
Env file: `docker/translation.env`  
Role: in-chat translation + Language Threads (`BOT__ROLE=translation`). No Whisper; needs `NEAR_AI_API_KEY`.

### Env + start

```bash
cp docker/translation.env.example docker/translation.env
# Edit: SIGNAL_PHONE = phone B (translation bot)
#       NEAR_AI_API_KEY = required
# Optional: PEER_PHONE = phone A when pairing with transcription

docker compose -f docker/compose.translation.yaml --env-file docker/translation.env up -d
```

First `up -d` builds `signal-bot` if needed. After later code changes, use [Code changes (rebuild the bot)](#code-changes-rebuild-the-bot) — plain `up -d` keeps the old binary.

Confirm network:

```bash
docker network ls | grep sigstack-translation
# Expect: sigstack-translation-internal
```

### Health

`signal-api` `/v1/health` returns **HTTP 204** with an empty body — no printed output and exit code 0 means healthy. Failure exits non-zero (`curl -sf`).

```bash
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env \
  exec signal-api curl -sf http://localhost:8080/v1/health
```

Registration proxy (host port **8081**):

```bash
curl -sf http://localhost:8081/health
```

### Register phone B

The number must match `SIGNAL_PHONE` in `docker/translation.env`.

Check whether the number is already registered with Signal CLI (skip captcha/register if it appears):

```bash
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env \
  exec signal-api curl -sS 'http://localhost:8080/v1/accounts'
# Expect JSON array, e.g. ["+1YYYYYYYYYY"]. Empty [] means not registered yet.
```

Or via the registration proxy:

```bash
curl -sS http://localhost:8081/v1/debug/signal-accounts
```

If not listed, generate a [captcha token](#captcha-token). Preferred register path: proxy on **localhost:8081**.

```bash
curl -sS -X POST "http://localhost:8081/v1/register/+1YYYYYYYYYY" \
  -H 'Content-Type: application/json' \
  -d '{"captcha":"signalcaptcha://PASTE_TOKEN_HERE","use_voice":false}'
```

Verify:

```bash
curl -sS -X POST "http://localhost:8081/v1/register/+1YYYYYYYYYY/verify/123456" \
  -H 'Content-Type: application/json' \
  -d '{}'
```

Alternatively, register against translation `signal-api` via compose `exec` (same pattern as phone A on the transcription stack).

Confirm again with `/v1/accounts` (or `v1/debug/signal-accounts`), then restart the bot so it picks up the registered session:

```bash
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env \
  restart signal-bot
```

### Logs

Idle bots are quiet at `info` — empty polls do not print. Incoming receive lines are mostly `debug`; successful command/handler work logs at `info`.

```bash
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env \
  logs -f signal-bot
```

Useful variants:

```bash
# All services on this stack
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env logs -f

# Last 100 lines, then follow
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env \
  logs -f --tail=100 signal-bot

# signal-api (registration / receive issues)
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env \
  logs -f signal-api
```

Raise verbosity: set `LOG_LEVEL=debug` in `docker/translation.env`, then recreate the bot:

```bash
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env \
  up -d signal-bot
```

### Stop / rebuild

Stop the stack (keeps Signal CLI volumes):

```bash
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env down
```

After code changes, rebuild/recreate — see [Code changes (rebuild the bot)](#code-changes-rebuild-the-bot). Do **not** use `down -v` unless you intend to wipe Signal CLI state (you will need to re-register the phone).

---

## Using both together

Confirm both networks exist:

```bash
docker network ls | grep sigstack
# Expect: sigstack-transcription-internal and sigstack-translation-internal
```

There is **no** Docker network between the two CVMs/stacks — Signal is the bus.

After both numbers are registered:

1. Create (or open) a Signal group that includes both bot numbers and your personal account.
2. For transcription pairing, set `PEER_PHONE` on translation to phone A and follow [voice-transcription.md](../voice-transcription.md#pairing-translation-leads) (`!transcription` as group admin).

## Related docs

| Doc | Why |
|-----|-----|
| [voice-transcription.md](../voice-transcription.md) | Transcription ops + pairing |
| [in-chat-translation.md](../in-chat-translation.md) | In-chat translate product |
| [language-threads.md](../language-threads.md) | Language Threads |
| [two-cvm-architecture.md](../two-cvm-architecture.md) | Why two stacks / no shared Docker network |
