#!/usr/bin/env bash
# Local mirror of every GitHub Actions check that can fail a PR/push.
#
# Workflows covered (only two in .github/workflows/):
#   1) .github/workflows/test.yml       → Format, Clippy, coverage gate
#   2) .github/workflows/commitlint.yml → conventional commits on the branch
#
# Intentionally NOT run (not in GitHub Actions):
#   - web ESLint / web build
#   - cargo build --release
#   - husky commit-msg (runs on `git commit`, not here)
#
# Usage:
#   npm run ci
#   pnpm run ci    # note: NOT `pnpm ci` (that is install-only)
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Prefer local binaries (pnpm/npm hoisted .bin) over global npx surprises.
export PATH="$ROOT/node_modules/.bin:$PATH"

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: missing '$1'. $2" >&2
    exit 1
  fi
}

need cargo "Install Rust from https://rustup.rs"
need rustfmt "Run: rustup component add rustfmt"
need cargo-clippy "Run: rustup component add clippy"
need cargo-llvm-cov "Run: cargo install cargo-llvm-cov --locked"
need node "Install Node.js 22+ (matches actions/setup-node)"

# Stable toolchains ship `llvm-tools-*`; older ones used `llvm-tools-preview-*`.
# GitHub Actions still requests llvm-tools-preview via rust-toolchain (rustup alias).
if ! rustup component list --installed 2>/dev/null | grep -qE '^llvm-tools(-preview)?-'; then
  echo "error: missing rustup component llvm-tools (needed for cargo-llvm-cov)." >&2
  echo "  Run: rustup component add llvm-tools" >&2
  exit 1
fi

if [[ ! -d node_modules ]] || ! command -v commitlint >/dev/null 2>&1; then
  echo "error: commitlint not available (node_modules missing or incomplete)." >&2
  echo "  Run: npm ci   or   pnpm install" >&2
  exit 1
fi

step() {
  echo ""
  echo "==> $1"
}

STARTED_AT="$(date +%s)"
echo "Local CI — mirroring GitHub Actions workflows: test.yml + commitlint.yml"

# ---------------------------------------------------------------------------
# .github/workflows/test.yml — job: rust
# ---------------------------------------------------------------------------

# Step: Format
step "test.yml / Format"
# Exact command from .github/workflows/test.yml
cargo fmt --all -- --check

# Step: Clippy
step "test.yml / Clippy"
# Exact command from .github/workflows/test.yml
cargo clippy --workspace --all-targets -- -D warnings

# Step: Test with coverage gate (≥90% lines)
step "test.yml / Test with coverage gate (≥90% lines)"
echo "Running full workspace tests under llvm-cov (same flags as Actions)..."
# Exact command from .github/workflows/test.yml
cargo llvm-cov --workspace --lcov --output-path lcov.info \
  --fail-under-lines 90 \
  --ignore-filename-regex '(^|/)main\.rs$'

# Belt-and-suspenders: some cargo-llvm-cov builds have been observed not exiting
# non-zero for --fail-under-lines. Enforce ≥90% from the LCOV we just wrote.
LINE_PCT="$(python3 - <<'PY'
lf = lh = 0
with open("lcov.info", encoding="utf-8") as f:
    for line in f:
        if line.startswith("LF:"):
            lf += int(line[3:])
        elif line.startswith("LH:"):
            lh += int(line[3:])
if lf <= 0:
    raise SystemExit("lcov.info missing LF totals")
print(f"{100.0 * lh / lf:.2f}")
PY
)"
echo "LCOV line coverage: ${LINE_PCT}% (gate ≥90%)"
python3 - <<PY
pct = float("${LINE_PCT}")
if pct < 90.0:
    raise SystemExit(f"error: line coverage {pct:.2f}% is below the 90% gate")
PY

# Upload LCOV is Actions-only (artifact). Locally we just leave lcov.info in-tree
# (gitignored).

# ---------------------------------------------------------------------------
# .github/workflows/commitlint.yml — job: commitlint
# ---------------------------------------------------------------------------

step "commitlint.yml / Lint commit messages"
# Actions PR path:  --from <pr.base.sha> --to <pr.head.sha>
# Actions push path: --from <before>     --to <sha>
#
# Local equivalent for a PR against main: merge-base(HEAD, origin/main)..HEAD
# (same commit set Actions checks on a typical PR). Override if needed:
#   CI_COMMITLINT_FROM=<sha> CI_COMMITLINT_TO=<sha> npm run ci
if [[ -n "${CI_COMMITLINT_FROM:-}" ]]; then
  FROM="$CI_COMMITLINT_FROM"
elif git rev-parse --verify origin/main >/dev/null 2>&1; then
  FROM="$(git merge-base HEAD origin/main)"
elif git rev-parse --verify main >/dev/null 2>&1; then
  FROM="$(git merge-base HEAD main)"
else
  FROM="$(git rev-list --max-parents=0 HEAD)"
fi
TO="${CI_COMMITLINT_TO:-HEAD}"

echo "commitlint: checking commits ${FROM}..${TO}"
# Same CLI as Actions (npx --no -- commitlint ...); use PATH binary for npm/pnpm.
commitlint --from "$FROM" --to "$TO" --verbose

ELAPSED="$(( $(date +%s) - STARTED_AT ))"
echo ""
echo "All GitHub Actions checks passed locally in ${ELAPSED}s."
echo "  matched: test.yml (Format, Clippy, coverage ≥90%)"
echo "  matched: commitlint.yml (conventional commits ${FROM}..${TO})"
echo "  skipped: Actions-only artifact upload of lcov.info"
echo "LCOV written to lcov.info"
