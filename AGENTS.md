# AGENTS.md

Short entrypoint for coding agents. Humans: see [README.md](README.md).

## Product

TEE-hosted Signal bots for **voice transcription** and **group translation** (not a general AI chat assistant). Two phone numbers / two bots in a Signal group; Signal is the bus — no Docker network between CVMs. Whisper stays on the transcription stack only.

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
# Two different SIGNAL_PHONE values; NEAR_AI_API_KEY in translation.env

docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env up -d
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env up -d
```

## Read next

| Doc | Why |
|-----|-----|
| [`.agents/docs/DEVELOPMENT.md`](.agents/docs/DEVELOPMENT.md) | TEE trust model, `BOT__ROLE`, Phala dual-CVM ops |
| [`docs/two-cvm-architecture.md`](docs/two-cvm-architecture.md) | Architecture diagram and compose/Phala split |
| [`docs/in-chat-translation.md`](docs/in-chat-translation.md) | In-chat (group) bilingual auto/manual translate |
| [`docs/parallel-translation.md`](docs/parallel-translation.md) | Parallel Translation product behavior |
| [`docs/language-threads.md`](docs/language-threads.md) | Language Threads product behavior |
| [`.agents/skills/`](.agents/skills/) | Vendored skills (Rust, Docker, Stripe) |
| [`.agents/rules/`](.agents/rules/) | Agent rules (e.g. commit message subjects) |

## Rules of thumb

- Required env: `BOT__ROLE=transcription|translation`
- Do not reintroduce tools, x402, or general chat paths
- Image digests stay pinned in compose for attestation
- Commits must pass [commitlint](https://github.com/conventional-changelog/commitlint) (`type: subject`); subject all lowercase, no trailing period, dashes not snake_case — see [`.agents/rules/commit-messages.md`](.agents/rules/commit-messages.md). Run `npm install` or `pnpm install` so husky `commit-msg` / `pre-push` hooks are active
- **CI style gates are not optional.** GitHub Actions (`test.yml` + `commitlint.yml`) fails on fmt, Clippy `-D warnings`, llvm-cov ≥90% lines, and conventional commits. Before finishing Rust work run `npm run ci` / `pnpm run ci` (never bare `pnpm ci`). Husky `pre-push` runs that script; `commit-msg` runs commitlint on each commit. Cursor auto-fmts `.rs` edits and re-prompts on stop if fmt/clippy would fail CI.
