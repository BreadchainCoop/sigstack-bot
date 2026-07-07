# Signal Bot TEE

Private AI Chat Proxy running in Trusted Execution Environment (TEE)

## Overview

This project implements a Signal bot that runs inside a Dstack-powered TEE (Intel TDX) and proxies user messages to NEAR AI Cloud's private inference API. The design creates a fully verifiable, end-to-end private AI chat experience.

## Architecture

```
[User] <--Signal E2E--> [TEE: Signal CLI + Bot] <--HTTPS--> [NEAR AI GPU TEE]
                              |
                        [In-memory only]
                        [Intel TDX protected]
```

- **Signal**: E2E encrypted messaging between user and bot
- **Dstack TEE**: Verifiable proxy execution with Intel TDX attestation
- **NEAR AI Cloud**: Private inference with GPU TEE (NVIDIA H100/H200) attestation

## Features

- End-to-end privacy from user device to AI inference
- Dual attestation (Intel TDX + NVIDIA GPU TEE)
- Cryptographic verification with user-provided challenges
- In-memory conversation storage (no external persistence)
- Group chat support with shared conversation context
- OpenAI-compatible API integration
- Tool use (web search, weather, calculator) and **Poa DAO task tools** — read an
  org's projects/tasks and, for authorized operators, create and manage tasks
  on-chain with a TEE-derived wallet ([docs/poa-integration.md](docs/poa-integration.md))

## Bot Commands

| Command | Description |
|---------|-------------|
| `!verify <challenge>` | Get TEE attestation with your challenge embedded in TDX quote |
| `!clear` | Clear conversation history |
| `!models` | List available AI models |
| `!help` | Show help message |

Any other message is sent to the AI for a response.

## Group Chat Support

The bot can be added to Signal group chats with shared conversation context:

| Context | Behavior |
|---------|----------|
| Direct Message | Personal conversation history per user |
| Group Chat | Shared history - all members see the same context |

**In groups:**
- All messages contribute to a shared conversation
- The AI can reference what other group members said
- `!clear` clears the entire group's conversation history
- `!verify` works the same (provides TEE attestation)

**Example:**
```
Alice: "My favorite color is blue"
Bob: "What's Alice's favorite color?"
Bot: "Alice mentioned her favorite color is blue"
```

## Poa DAO Tools

When `TOOLS__POA__ENABLED=true`, the bot can operate a [Poa](https://github.com/poa-box)
DAO org's task board. Ask it things like "what open tasks are there?" or "create a
5 PT task in the Docs project to fix the changelog".

- **Read tools** (everyone): projects, tasks, proposals, wallet info — backed by
  the Poa subgraph.
- **Write tools** (allowlisted operators only), signed by a wallet **derived
  inside the TEE** (no key leaves the enclave):
  - *task authoring*: create / update / assign / complete / reject / cancel task
  - *participation*: claim / submit / apply — the bot does work and **earns** PT
  - *governance*: create non-executable polls, vote

Extra safety on top of the allowlist:

- **Confirmation** — value-moving actions (`poa_complete_task`, which mints a
  payout) are staged and require a deterministic `!poa-confirm <code>` reply from
  the same sender; the LLM cannot self-confirm.
- **Board steward** — an optional background loop posts a digest of expired /
  at-risk claims to a Signal group (read-only; never sends transactions).

Writes are gated by `TOOLS__POA__ENABLE_WRITES` plus a
`TOOLS__POA__AUTHORIZED_SENDERS` allowlist, and the chain enforces that the bot's
wallet holds the right TaskManager permission. Full setup — including how to grant
the wallet project-manager rights on the Poa side — is in
[docs/poa-integration.md](docs/poa-integration.md).

## Verification

Users can cryptographically verify the bot runs in a genuine TEE:

1. Send `!verify my-random-nonce` to the bot
2. Bot returns a TDX quote with your nonce embedded in `report_data`
3. Verify the quote signature at https://proof.phala.network
4. Compare `compose_hash` with this repository's `docker/docker-compose.yaml`

This proves:
- The attestation was generated fresh (contains your nonce)
- The code is running in Intel TDX hardware
- The exact docker-compose configuration is as published

## Project Structure

```
crates/
  signal-bot/          # Main application binary
  near-ai-client/      # NEAR AI Cloud API client
  conversation-store/  # In-memory conversation storage with TTL
  dstack-client/       # Dstack TEE attestation client
  signal-client/       # Signal CLI REST API client
docker/
  Dockerfile           # Multi-stage build for Alpine
  docker-compose.yaml  # Production deployment config
```

## Security Model

See [CLAUDE.md](./CLAUDE.md) for detailed security documentation including:

- Why in-memory storage instead of Redis
- Why Signal CLI must run inside the TEE
- User verification process
- Trust assumptions and metadata leakage

## Quick Start

### Prerequisites

- Rust 1.83+
- Docker & Docker Compose
- Signal phone number (for the bot)
- NEAR AI API key

### Build

```bash
cargo build --release
```

### Test

```bash
cargo test
```

### Deploy

```bash
cd docker
cp ../.env.example .env
# Edit .env with your credentials
docker-compose up -d
```

## Multitenant Registration Proxy

The signal-registration-proxy provides a secure API for registering multiple Signal phone numbers. Each tenant (phone number) has isolated conversation history and can be managed independently.

### Registration API

Base URL: `https://[your-deployment]-8081.dstack-prod5.phala.network`

#### Register a Phone Number

Initiates registration and sends SMS verification code.

```bash
curl -X POST https://[base-url]/v1/register/+1234567890 \
  -H "Content-Type: application/json" \
  -d '{
    "captcha": "signalcaptcha://signal-hcaptcha...",
    "use_voice": false,
    "ownership_secret": "your-secret-for-verification"
  }'
```

**Parameters:**
- `captcha` (optional): Captcha token from [signalcaptchas.org](https://signalcaptchas.org/registration/generate.html) - required if Signal requests it
- `use_voice` (optional): `true` for voice call instead of SMS
- `ownership_secret` (optional): Secret to prove ownership for future operations

**Response:**
```json
{
  "phone_number": "+1234567890",
  "status": "pending",
  "message": "Verification code sent. Use /v1/register/{number}/verify/{code} to complete."
}
```

#### Verify Registration

Submit the SMS/voice verification code.

```bash
curl -X POST https://[base-url]/v1/register/+1234567890/verify/123456 \
  -H "Content-Type: application/json" \
  -d '{
    "ownership_secret": "your-secret-for-verification",
    "pin": "optional-signal-pin"
  }'
```

**Parameters:**
- `ownership_secret`: Must match the secret used during registration
- `pin` (optional): Signal PIN if the account has one set

#### Check Registration Status

```bash
curl https://[base-url]/v1/status/+1234567890
```

**Response:**
```json
{
  "phone_number": "+1234567890",
  "status": "verified",
  "registered_at": "2025-01-15T10:30:00Z"
}
```

#### List All Registered Accounts

```bash
curl https://[base-url]/v1/accounts
```

**Response:**
```json
{
  "accounts": [
    {
      "phone_number": "+1234567890",
      "status": "verified",
      "registered_at": "2025-01-15T10:30:00Z"
    }
  ],
  "total": 1
}
```

#### Unregister a Phone Number

```bash
curl -X DELETE https://[base-url]/v1/unregister/+1234567890 \
  -H "Content-Type: application/json" \
  -d '{"ownership_secret": "your-secret-for-verification"}'
```

### Health Check

```bash
curl https://[base-url]/health
```

**Response:**
```json
{
  "status": "ok",
  "registry_count": 1,
  "signal_api_healthy": true
}
```

### Multitenant Isolation

Each registered phone number is a separate tenant with:

- **Isolated conversations**: Each phone number has its own conversation history
- **Separate storage**: Registry entries encrypted with TEE-derived keys
- **Rate limiting**: Per-number rate limits prevent abuse
- **Ownership verification**: Operations require the secret used at registration

### Registration Troubleshooting

If registration fails with "Account is already registered":
1. The Signal CLI may have stale data from a previous registration
2. Use the debug endpoint to force unregister: `POST /v1/debug/force-unregister/+1234567890`
3. Retry registration with a fresh captcha

See [CLAUDE.md](./CLAUDE.md) for detailed debugging documentation.

## Configuration

Environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `SIGNAL__PHONE_NUMBER` | Bot's Signal phone number | Required |
| `SIGNAL__SERVICE_URL` | Signal CLI REST API URL | `http://signal-api:8080` |
| `NEAR_AI__API_KEY` | NEAR AI API key | Required |
| `NEAR_AI__MODEL` | AI model to use | `llama-3.3-70b` |
| `CONVERSATION__TTL` | Conversation expiry time | `24h` |
| `CONVERSATION__MAX_MESSAGES` | Max messages per conversation | `50` |

## License

MIT
