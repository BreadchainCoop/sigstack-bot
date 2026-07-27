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
npm run prepush          # lint (rust+web) + coverage gate + commitlint:branch

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
| [`docs/language-threads.md`](docs/language-threads.md) | Language Threads product behavior |
| [`.agents/skills/`](.agents/skills/) | Vendored skills (Rust, Docker, Stripe) |

## Rules of thumb

- Required env: `BOT__ROLE=transcription|translation`
- Do not reintroduce tools, x402, or general chat paths
- Image digests stay pinned in compose for attestation
- Commits must pass [commitlint](https://github.com/conventional-changelog/commitlint) (`type: subject`); run `npm install` so husky `commit-msg` / `pre-push` hooks are active
- **CI style gates are not optional.** GitHub Actions fails on `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` (Clippy style/idiom lints, not broken tests). Before finishing Rust work: `npm run lint:rust` (fast) or `npm run ci` (full Actions parity). Husky `pre-push` runs `npm run ci`. Cursor auto-runs `cargo fmt` on `.rs` edits and re-prompts on stop if fmt/clippy would fail CI.
