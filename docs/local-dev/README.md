# Local development

Thorough guide for running the dual Compose stacks on your machine, registering Signal phone numbers (including captcha), and tailing logs.

Quick start (env copy + `up -d`) stays in [README.md](../README.md#local-dual-stack). Architecture context: [two-cvm-architecture.md](../two-cvm-architecture.md). TEE / role details: [`.agents/docs/DEVELOPMENT.md`](../../.agents/docs/DEVELOPMENT.md).

## Prerequisites

- Docker with Compose v2
- Two different Signal-capable phone numbers (E.164), one per stack
- `NEAR_AI_API_KEY` in `docker/translation.env` for the translation bot
- Optional: a local `/var/run/dstack.sock` if you care about attestation paths; local Compose mounts it read-only — registration and day-to-day bot traffic do not require a live Phala socket

## 1. Start both stacks

```bash
cp docker/transcription.env.example docker/transcription.env
cp docker/translation.env.example docker/translation.env
# Edit: two different SIGNAL_PHONE values; PEER_PHONE cross-links; NEAR_AI_API_KEY in translation.env

docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env up -d
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env up -d
```

Confirm networks:

```bash
docker network ls | grep sigstack
# Expect: sigstack-transcription-internal and sigstack-translation-internal
```

Health (transcription stack):

```bash
docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env \
  exec whisper-api curl -sf http://localhost:9000/health
docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env \
  exec signal-api curl -sf http://localhost:8080/v1/health
```

Translation `signal-api` health:

```bash
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env \
  exec signal-api curl -sf http://localhost:8080/v1/health
```

Registration proxy (translation stack only, host port **8081**):

```bash
curl -sf http://localhost:8081/health
```

## 2. Register phone numbers (Signal captcha)

Each stack has its own `signal-api` and its own Signal CLI data volume. Register **phone A** against the transcription stack and **phone B** against the translation stack. The numbers must match `SIGNAL_PHONE` in the corresponding env file.

### Captcha token

Signal almost always requires a captcha before SMS/voice verification:

1. Open [Signal registration captcha](https://signalcaptchas.org/registration/generate.html) in a browser.
2. Solve the challenge.
3. The page redirects to a `signalcaptcha://…` URL. Copy the **entire** token string (starts with `signalcaptcha://`).
   - If the browser does not show the token clearly, open DevTools → Network/Console, or right-click the failed-navigation link and copy the URL.
4. Use that string as the `captcha` field in the register request below. Do not strip the `signalcaptcha://` prefix.

Tokens expire quickly — generate a fresh one if registration fails with a captcha error.

### Phone A — transcription stack (`signal-api` via compose exec)

`signal-api` is not published on the host; call it from inside the container.

Replace `+1XXXXXXXXXX` with the transcription `SIGNAL_PHONE`.

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

Confirm the account is present:

```bash
docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env \
  exec signal-api curl -sS 'http://localhost:8080/v1/accounts'
```

Restart the transcription bot so it picks up the registered session:

```bash
docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env \
  restart signal-bot
```

### Phone B — translation stack (registration proxy on `:8081`)

Preferred for phone B: the translation compose exposes `signal-registration-proxy` on **localhost:8081**.

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

Alternatively, register against translation `signal-api` the same way as phone A (compose `exec`), then restart `signal-bot` on the translation stack.

```bash
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env \
  restart signal-bot
```

### After both numbers are registered

1. Create (or open) a Signal group that includes both bot numbers and your personal account.
2. For transcription pairing, set `PEER_PHONE` on translation to phone A and follow [voice-transcription.md](../voice-transcription.md#pairing-translation-leads) (`!transcription` as group admin).

## 3. Monitor logs in the terminal

Follow the bot process you care about (`-f` = follow):

```bash
# Transcription bot
docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env \
  logs -f signal-bot

# Translation bot
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env \
  logs -f signal-bot
```

Useful variants:

```bash
# All services on one stack
docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env logs -f

# Last 100 lines, then follow
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env \
  logs -f --tail=100 signal-bot

# signal-api only (registration / receive issues)
docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env \
  logs -f signal-api
```

Raise verbosity with `LOG_LEVEL=debug` in the env file and recreate the bot container:

```bash
# Example: translation
# Set LOG_LEVEL=debug in docker/translation.env, then:
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env \
  up -d signal-bot
```

## 4. Stop / rebuild

```bash
docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env down
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env down
```

Rebuild after code changes (example: translation bot image):

```bash
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env \
  build signal-bot
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env \
  up -d signal-bot
```

Do **not** use `down -v` unless you intend to wipe Signal CLI state (you will need to re-register the phones).

## Related docs

| Doc | Why |
|-----|-----|
| [voice-transcription.md](../voice-transcription.md) | Transcription ops + pairing |
| [in-chat-translation.md](../in-chat-translation.md) | In-chat translate product |
| [language-threads.md](../language-threads.md) | Language Threads |
| [two-cvm-architecture.md](../two-cvm-architecture.md) | Why two stacks / no shared Docker network |
