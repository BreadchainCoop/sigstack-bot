#!/usr/bin/env bash
# Run every check that GitHub Actions can fail on this repo.
# Mirrors:
#   .github/workflows/test.yml       (fmt → clippy → coverage gate)
#   .github/workflows/commitlint.yml (conventional commits on this branch)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

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
need node "Install Node.js (https://nodejs.org)"
need npm "Install npm (comes with Node.js)"

# Stable toolchains ship `llvm-tools-*`; older ones used `llvm-tools-preview-*`.
if ! rustup component list --installed 2>/dev/null | grep -qE '^llvm-tools(-preview)?-'; then
  echo "error: missing rustup component llvm-tools (needed for cargo-llvm-cov)." >&2
  echo "  Run: rustup component add llvm-tools" >&2
  exit 1
fi

if [[ ! -d node_modules ]]; then
  echo "error: root node_modules missing (needed for commitlint)." >&2
  echo "  Run: npm ci" >&2
  exit 1
fi

step() {
  echo ""
  echo "==> $1"
}

STARTED_AT="$(date +%s)"

# --- .github/workflows/test.yml ---

step "Format (cargo fmt --check)"
cargo fmt --all -- --check

step "Clippy (-D warnings)"
cargo clippy --workspace --all-targets -- -D warnings

step "Test with coverage gate (≥90% lines)"
echo "Running full workspace test suite under llvm-cov (this usually takes minutes)..."
# Same flags as .github/workflows/test.yml (llvm-cov runs the test suite)
cargo llvm-cov --workspace --lcov --output-path lcov.info \
  --fail-under-lines 90 \
  --ignore-filename-regex '(^|/)main\.rs$'

# --- .github/workflows/commitlint.yml ---

step "Commitlint (branch commits)"
# Local equivalent of the PR path: lint commits since merge-base with main
if git rev-parse --verify origin/main >/dev/null 2>&1; then
  FROM="$(git merge-base HEAD origin/main)"
elif git rev-parse --verify main >/dev/null 2>&1; then
  FROM="$(git merge-base HEAD main)"
else
  FROM="$(git rev-list --max-parents=0 HEAD)"
fi
echo "commitlint: checking commits ${FROM}..HEAD"
npx --no -- commitlint --from "$FROM" --to HEAD --verbose

ELAPSED="$(( $(date +%s) - STARTED_AT ))"
echo ""
echo "All GitHub Actions checks passed locally in ${ELAPSED}s."
echo "  - test.yml: format, clippy, coverage (≥90%)"
echo "  - commitlint.yml: conventional commit messages"
echo "LCOV written to lcov.info"
