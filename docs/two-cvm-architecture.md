# Two-CVM architecture

Product suite split across two Phala CVMs (and two local Docker Compose projects). Signal group chat is the only cross-stack bus.

See also: [issue #10](https://github.com/BreadchainCoop/sigstack-bot/issues/10).

## Diagram

```mermaid
flowchart LR
  subgraph group [Signal_group]
    Users[Human_members]
  end
  subgraph txCVM [Transcription_CVM_4GB]
    txApi[signal-api_phone_A]
    whisper[whisper-api]
    txBot[signal-bot_role_transcription]
    txBot -->|"HTTP same Docker net"| whisper
    txApi --- txBot
  end
  subgraph trCVM [Translation_CVM_4GB]
    trApi[signal-api_phone_B]
    trBot[signal-bot_role_translation]
    trApi --- trBot
  end
  Users <-->|Signal_all_messages| txApi
  Users <-->|Signal_all_messages| trApi
  txBot -->|acts_on_voice_only_posts_text| group
  trBot -->|acts_on_text_incl_transcripts| group
```

## Rules

- **Two phone numbers**, two bots in the group.
- Signal delivers **all** group messages to every member bot. The transcription bot **receives** text but **ignores** it; it only **acts** on voice. The translation bot receives voice too but only **acts** on text (including transcripts posted by the transcription bot).
- No cross-CVM Docker/HTTP link. Whisper stays **inside** the transcription stack only.
- Same `signal-bot` image; role selected by `BOT__ROLE=transcription|translation`.

## Local dual stack (mock prod)

```bash
cp docker/transcription.env.example docker/transcription.env
cp docker/translation.env.example docker/translation.env
# Set two different SIGNAL_PHONE values; set NEAR_AI_API_KEY in translation.env

docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env up -d
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env up -d

docker network ls | grep sigstack
# Expect: sigstack-transcription-internal and sigstack-translation-internal
```

Register each number against its own stack’s `signal-api` (e.g. `docker compose … exec signal-api curl …`). Translation compose also exposes the registration proxy on host port `8081`.

E2E still needs two registered Signal numbers in a shared test group. Dual compose mocks **isolation and resource split**, not Signal itself.

## Phala (prod)

| Compose | CVM | Contents |
|---------|-----|----------|
| [`docker/phala.transcription.yaml`](../docker/phala.transcription.yaml) | 4 GB (`tdx.medium`) | `signal-api` + `whisper-api` + `signal-bot` (`BOT__ROLE=transcription`) |
| [`docker/phala.translation.yaml`](../docker/phala.translation.yaml) | 4 GB (`tdx.medium`) | `signal-api` + `signal-bot` (`BOT__ROLE=translation`) + registration proxy |

Deploy each compose to its **own** CVM. Do not co-locate Whisper with the translation bot.

## Products on each CVM

| Product | CVM |
|---------|-----|
| Voice transcription | Transcription |
| In-chat (group) translation | Translation |
| Parallel Translation (not implemented yet) | Translation |
| Language Threads | Translation |

## Why split

Transcription (Whisper) is latency- and memory-heavy. Keeping it on a separate CVM prevents long voice jobs from queuing behind translation traffic for users who only subscribe to translation.
