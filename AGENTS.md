# AGENTS.md

Short entrypoint for coding agents. Humans: see [README.md](README.md).

## Product

TEE-hosted Signal bots for **voice transcription** and **group translation** (not a general AI chat assistant). Two phone numbers / two bot processes in a Signal group on **one** Phala CVM. STT is **NEAR AI Whisper Large V3** (GPU TEE) — do not reintroduce an in-CVM Whisper sidecar. See [`docs/solutions/architecture-patterns/2026-08-13-cpu-tee-whisper-does-not-scale.md`](docs/solutions/architecture-patterns/2026-08-13-cpu-tee-whisper-does-not-scale.md).

## Compound Engineering

This repo uses [Compound Engineering](https://every.to/guides/compound-engineering) via the Cursor marketplace plugin (not vendored). Each unit of work should make the next unit easier.

### Install / first run (manual)

In Cursor Agent chat:

```text
/add-plugin compound-engineering
/ce-setup
```

`/ce-setup` should report gitignore coverage for machine-local config, a present `.compound-engineering/config.local.example.yaml`, and no obsolete local md. Optional tools (`gh`, `ast-grep`, etc.) are informational only.

### Core loop

```text
/ce-brainstorm → /ce-plan → /ce-work → /ce-simplify-code → /ce-code-review → /ce-compound
```

| Artifact | Path |
|----------|------|
| Brainstorms | [`docs/brainstorms/`](docs/brainstorms/) |
| Plans | [`docs/plans/`](docs/plans/) |
| Solutions (compounded learnings) | [`docs/solutions/`](docs/solutions/) |
| Triage / review todos | [`todos/`](todos/) |

After non-trivial fixes or features, prefer `/ce-compound` so the next cycle starts smarter. Domain skills (Rust, Docker, Stripe) under [`.agents/skills/`](.agents/skills/) coexist with CE plugin skills — use CE for feature workflow, domain skills for language/stack craft.

## Setup / verify

```bash
cargo test
cargo build --release -p signal-bot

# Coverage (requires cargo-llvm-cov + rustup component llvm-tools)
npm test                 # cargo test --workspace
npm run test:cov:report  # summary only (ignores main.rs)
npm run test:cov:ci      # LCOV + fail under 90% lines (same gate as CI / prepush)
npm run ci               # all GitHub Actions gates (fmt + clippy + coverage + commitlint)
pnpm run ci              # same as above if you use pnpm (NOT `pnpm ci` — that only installs)
npm run prepush          # alias of npm run ci (also run by husky pre-push)

cp docker/transcription.env.example docker/transcription.env
cp docker/translation.env.example docker/translation.env
# Two different SIGNAL_PHONE values; NEAR_AI_API_KEY in both (STT + chat)

docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env up -d
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env up -d
```

## Read next

| Doc | Why |
|-----|-----|
| [`.agents/docs/DEVELOPMENT.md`](.agents/docs/DEVELOPMENT.md) | TEE trust model, `BOT__ROLE`, Phala one-CVM ops, **CVM volume / Signal identity** |
| [`docs/two-cvm-architecture.md`](docs/two-cvm-architecture.md) | One CVM / two phones, **CVM storage (keep intact)** |
| [`docs/voice-transcription.md`](docs/voice-transcription.md) | Voice transcription product + pairing (NEAR Whisper) |
| [`docs/solutions/architecture-patterns/2026-08-13-cpu-tee-whisper-does-not-scale.md`](docs/solutions/architecture-patterns/2026-08-13-cpu-tee-whisper-does-not-scale.md) | Why STT is remote; never re-home Whisper in a CPU TEE |
| [`docs/in-chat-translation.md`](docs/in-chat-translation.md) | In-chat (group) bilingual auto/manual translate |
| [`docs/language-threads.md`](docs/language-threads.md) | Language Threads (multilingual main + N sidecars) |
| [`docs/solutions/`](docs/solutions/) | Compounded learnings from prior work |
| [`docs/plans/`](docs/plans/) | CE implementation plans |
| [`.agents/skills/`](.agents/skills/) | Domain skills (Rust, Docker, Stripe) |
| [`.cursor/rules/`](.cursor/rules/) | Cursor project rules (commits, compound loop, **CVM storage**) |

## CVM storage (do not wipe)

**Never destroy live Phala volumes or replace the registered translation CVM for a routine upgrade.** Prod is **one** CVM (`0e82fa77-8b15-4dbd-89c4-9045ab911353`). Keep:

1. **Registered Signal phones** (`signal-config-translation` = phone B; `signal-config-transcription` = phone A) — losing a volume unlinks that bot until ops re-registers (and takes over the number).
2. **Encrypted user prefs** (`group-prefs-*` → `/data/group_prefs.enc`) — `!translate-me-on`, `!translate-all-on`, Language Threads bridges. Losing this forces every user to turn features back on.

Upgrade with `phala deploy --cvm-id 0e82fa77-8b15-4dbd-89c4-9045ab911353`. Do not create a new CVM, rename those volumes, or `down -v`. Do not recreate a transcription CVM for Whisper. TEE RAM wipe on reboot is expected; disk volumes are the identity. Details: [docs/two-cvm-architecture.md — CVM storage](docs/two-cvm-architecture.md#cvm-storage-keep-intact).

## Rules of thumb

- Required env: `BOT__ROLE=transcription|translation`
- Do not reintroduce tools, x402, or general chat paths
- Do not reintroduce a local Whisper sidecar; STT is NEAR AI Whisper Large V3
- Image digests stay pinned in compose for attestation
- **Keep CVM volumes and Signal registrations** — in-place Phala upgrades only; see [CVM storage](#cvm-storage-do-not-wipe)
- Commits must pass [commitlint](https://github.com/conventional-changelog/commitlint) (`type: subject`); subject all lowercase, no trailing period, dashes not snake_case — see [`.cursor/rules/commit-messages.mdc`](.cursor/rules/commit-messages.mdc). Run `npm install` or `pnpm install` so husky `commit-msg` / `pre-push` hooks are active
- **CI style gates are not optional.** GitHub Actions (`test.yml` + `commitlint.yml`) fails on fmt, Clippy `-D warnings`, llvm-cov ≥90% lines, and conventional commits. Before finishing Rust work run `npm run ci` / `pnpm run ci` (never bare `pnpm ci`). Husky `pre-push` runs that script; `commit-msg` runs commitlint on each commit. Cursor auto-fmts `.rs` edits and re-prompts on stop if fmt/clippy would fail CI.
