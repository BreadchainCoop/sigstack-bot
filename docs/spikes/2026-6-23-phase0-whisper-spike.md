# Phase 0 Spike: Whisper + Signal Attachments

**Date:** 2026-06-23  
**Status:** Complete (pending live voice-note JSON capture from user)  
**Plan:** `docs/plans/2026-6-22-whisper-integration.md`

## Summary

| Spike item | Result |
|------------|--------|
| Voice attachment JSON on `/v1/receive` | Documented from swagger + fixtures; live inbox empty |
| Attachment download API | `GET /v1/attachments/{id}` — no auth on internal network |
| Quote-reply JSON (inbound) | `dataMessage.quote` with `id`, `authorNumber`, `text` |
| Quote-reply send API | `POST /v2/send` with `quote_timestamp`, `quote_author`, `quote_message` |
| Whisper runtime | **whisper.cpp `whisper-server`** in `ghcr.io/ggerganov/whisper.cpp:main` |
| faster-whisper | **Not selected** — Python/CUDA stack; heavier for CPU-only Phala CVM |

## 1. Signal attachment receive JSON

### Endpoint

```
GET /v1/receive/{number}?timeout=1
```

Query params (optional): `ignore_attachments`, `ignore_stories`, `max_messages`, `send_read_receipts`.

Response: JSON array of `{ envelope, account }` objects (same shape as existing `IncomingMessage` in `signal-client`).

### Voice note shape

Voice notes arrive on `envelope.dataMessage` with:

- `message`: `null` (no text body)
- `attachments[]`: at least one entry with `contentType` `audio/ogg` or `audio/aac`
- `id`: attachment ID used for download (e.g. `pwtcq-xxxx`)
- `size`, `uploadTimestamp`, optional `filename`

See fixtures:

- `docs/spikes/fixtures/voice-note-dm.json`
- `docs/spikes/fixtures/voice-note-group.json`

**Source:** [signal-cli-rest-api swagger](https://bbernhard.github.io/signal-cli-rest-api/) (`receive.Attachment`, `receive.DataMessage`), [issue #52](https://github.com/bbernhard/signal-cli-rest-api/issues/52).

### Live capture (needs user)

Local `signal-api` is running; `/v1/receive/+573107677679` returned `[]` (empty inbox).

**To validate fixtures against production JSON:** send a short voice note to `+573107677679`, then:

```bash
docker exec signal-api curl -s "http://localhost:8080/v1/receive/%2B573107677679?timeout=5" | jq .
```

Save output to `docs/spikes/fixtures/voice-note-dm-live.json` and diff against fixture.

## 2. Attachment download

| Method | Path | Auth |
|--------|------|------|
| `GET` | `/v1/attachments` | None (internal Docker network) |
| `GET` | `/v1/attachments/{attachment}` | None — returns raw bytes |
| `DELETE` | `/v1/attachments/{attachment}` | None |

Attachments are auto-downloaded during `receive` unless `ignore_attachments=true`.

**Bot flow:**

1. Parse `attachments[].id` from receive JSON
2. `GET http://signal-api:8080/v1/attachments/{id}` → audio bytes in memory
3. POST to whisper sidecar; drop bytes after transcribe

No API key or account header required — same trust boundary as existing `signal-bot` → `signal-api` calls.

## 3. Quote-reply metadata

### Inbound (`!translate` detection)

On `dataMessage`:

```json
"quote": {
  "id": 1718999999000,
  "author": "<uuid>",
  "authorNumber": "+14155559876",
  "authorUuid": "<uuid>",
  "text": "quoted message body or transcript",
  "attachments": []
}
```

Fixture: `docs/spikes/fixtures/text-with-quote-reply.json`

`quote.text` is sufficient for `!translate` — no in-bot cache needed.

### Outbound (bot quote-replies transcript/translation)

`POST /v2/send` body fields (from `api.SendMessageV2`):

| Field | Type | Purpose |
|-------|------|---------|
| `quote_timestamp` | integer | `id` from quoted message (or message timestamp) |
| `quote_author` | string | Original sender phone or UUID |
| `quote_message` | string | Snippet shown in quote bubble (optional but recommended) |
| `message` | string | Bot reply body |
| `number` | string | Bot account |
| `recipients` | string[] | DM source or `groupId` |

Example sketch:

```json
{
  "number": "+573107677679",
  "recipients": ["+14155559876"],
  "message": "📝 Transcript:\nHola...",
  "quote_timestamp": 1719000000000,
  "quote_author": "+14155559876",
  "quote_message": ""
}
```

**Phase 1 action:** extend `SendMessageRequest` / `SignalClient::reply_quoted()`.

## 4. Whisper sidecar benchmark

**Platform:** `linux/amd64` (Phala CVM target)  
**Image:** `ghcr.io/ggerganov/whisper.cpp:main`  
**Digest (pulled 2026-06-23):** `sha256:13d0e7c873c59a262dca621b57ad28de40e9927ac883d936dfd8459142c90db4`

### Model

| Model | File size | Notes |
|-------|-----------|-------|
| `small` | 466 MB | PoC default per plan |

Download: `./models/download-ggml-model.sh small /models`

### CLI benchmark (jfk.wav ≈ 11 s speech, 2 vCPU emulated amd64)

| Task | whisper.cpp `total time` |
|------|--------------------------|
| `transcribe` (`-nt`) | **4.7 s** |
| `translate` (`-nt -tr`) | **4.5 s** |

Extrapolation: ~30 s audio ≈ 13 s CPU on `small` — well within `WHISPER__TIMEOUT=120s`.

### HTTP server (`whisper-server`)

Binary: `./build/bin/whisper-server` (included in official image)

```bash
./build/bin/whisper-server -m /models/ggml-small.bin --host 0.0.0.0 --port 9000
```

| Endpoint | Method | Notes |
|----------|--------|-------|
| `/health` | GET | `{"status":"ok"}` |
| `/inference` | POST | `multipart/form-data`, field `file=@audio.wav`, `response_format=json` |

Verified response:

```json
{"text":" And so my fellow Americans, ask not what your country can do for you, ask what you can do for your country.\n"}
```

Server includes ffmpeg for format conversion (`WHISPER_CONVERT` equivalent) — handles Signal `audio/ogg` / `audio/aac`.

### faster-whisper (not selected)

| Criterion | whisper.cpp | faster-whisper |
|-----------|-------------|----------------|
| Runtime | C++ only | Python + CTranslate2 |
| Docker | Official `ghcr.io/ggerganov/whisper.cpp` | Heavier; GPU images common |
| Phala CPU TEE | Good fit | Extra attack surface / RAM |
| HTTP API | Built-in `whisper-server` | Community wrappers |
| Translate task | Native `-tr` flag | Separate pipeline |

**Decision:** Build `docker/Dockerfile.whisper` from `ghcr.io/ggerganov/whisper.cpp:main`, bake `small` model, run `whisper-server` on port 9000.

## 5. Proposed sidecar compose (Phase 2)

```yaml
whisper-api:
  image: <our-built-image>@sha256:...
  platform: linux/amd64
  environment:
    - WHISPER_MODEL=small
  networks:
    - internal
  # No ports exposed publicly
  healthcheck:
    test: ["CMD", "curl", "-f", "http://localhost:9000/health"]
```

`signal-bot` env: `WHISPER__SERVICE_URL=http://whisper-api:9000`

Client wraps `POST /inference` with multipart upload (not base64 JSON — matches server native API).

## 6. Open after Phase 0

- [ ] **User:** send voice note to bot → save live receive JSON
- [ ] **User:** quote-reply `!translate es` on transcript → save live quote JSON
- [ ] Confirm `quote_timestamp` mapping with real Signal timestamps (use envelope `dataMessage.timestamp` of quoted msg)

## 7. Ready for Phase 1

- Extend `DataMessage` / `BotMessage` using fixtures
- Add `download_attachment(id) -> Vec<u8>`
- Add `send_quoted(...)` on `SignalClient`
- No Whisper code in Phase 1 — log attachment bytes length only
