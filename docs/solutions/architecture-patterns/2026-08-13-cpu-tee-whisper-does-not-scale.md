---
title: CPU TEE Whisper does not scale
date: 2026-08-13
category: architecture-patterns
module: transcription
problem_type: architecture_pattern
component: documentation
severity: high
applies_when:
  - "choosing where voice transcription inference runs"
  - "sizing a Phala TDX CVM for Signal bots"
  - "considering a local whisper-api sidecar in compose"
tags: [whisper, phala, tdx, near-ai, concurrency]
---

# CPU TEE Whisper does not scale

## Context

Dual-CVM was supposed to give Whisper room. It did not. Live `tdx.medium` (2 vCPU / 4 GB) still made ~5s Signal voice notes feel unusable, and overlapping subscribers queued on one CPU-bound `whisper.cpp`. Phala TDX SKUs couple **1 vCPU / 2 GB**, so “buy more RAM for a bigger model” also buys cores you still cannot turn into real parallel Whisper without paying 2×/4× for `large`/`xlarge`. A bigger ggml model on the same box is **slower**, not faster (accuracy vs compute). Extra Whisper workers on 2 vCPU split the same cores and make the first user worse. TDX has no GPU; CPU Whisper will not become production-grade by resizing the CVM. Meanwhile a dedicated transcription TEE billed ~$0.116/hr whether anyone was speaking.

## Guidance

Decrypt voice notes in the Signal TEE, strip Signal metadata, and send **audio bytes only** to **NEAR AI Whisper Large V3** (`POST /v1/audio/transcriptions`, model `openai/whisper-large-v3`) in their GPU TEE — the same vendor already used for translation text.

Keep **one** bot process on **one** Phala CVM (`tdx.medium`). Per-message `tokio::spawn` keeps translation off the STT wait. Do **not** add `whisper-api` back to compose. Do **not** put Whisper on a larger CPU TEE as the scale path.

Outbound STT is a multipart file plus model name. Generic filename (`voice.ogg` / `voice.m4a`). No phone, group id, Signal timestamp, or display name in form fields, headers, or filenames.

## Why This Matters

Local Whisper on 2 vCPU cannot run real parallel jobs. Isolation across two CVMs only stopped translation from sharing RAM with Whisper; it did not make inference fast. Remote GPU STT is the latency lever. A dedicated transcription CVM was idle cost with no product win.

Two processes used to matter because each bot awaited `dispatch_message` inside its own poll loop, and a second Signal number let translation *see* transcripts. One number does not receive its own group sends, so transcripts fan out in-process; `tokio::spawn` per inbound message replaces the second process for latency isolation. Shared CVM contention is mild I/O (one Java CLI + one Rust bot waiting on NEAR), not a CPU Whisper queue.

## When to Apply

- Any change that would reintroduce an in-CVM Whisper sidecar
- CVM sizing discussions for transcription latency
- Privacy copy (`!privacy`, `!help-transcription`) and attestation (`!verify` attests **this** CVM’s compose, not remote Whisper weights)

## Examples

**Do**

- `WHISPER__SERVICE_URL=https://cloud-api.near.ai/v1` with `NEAR_AI__API_KEY`
- One compose: [`docker/phala.translation.yaml`](../../../docker/phala.translation.yaml) — one `signal-api` + one `signal-bot` (`BOT__ROLE=translation`)
- In-place upgrades: `phala deploy --cvm-id 0e82fa77-8b15-4dbd-89c4-9045ab911353`

**Do not**

- Add `whisper-api` / `Dockerfile.whisper` to live compose
- Deploy `docker/phala.transcription.yaml` (deprecated stub)
- “Just use `medium`/`large` ggml” or `tdx.large`/`xlarge` to fix voice latency
- Re-introduce a second Signal number / pairing, or re-home Whisper in this CVM

## Related

- [`docs/two-cvm-architecture.md`](../../two-cvm-architecture.md)
- [`docs/voice-transcription.md`](../../voice-transcription.md)
- [`AGENTS.md`](../../../AGENTS.md)
- Historical dual-CVM deploy: [`docs/plans/2026-08-05-dual-cvm-phala-deploy.md`](../../plans/2026-08-05-dual-cvm-phala-deploy.md) (superseded)
