#!/usr/bin/env bash
# On agent stop: if Rust sources changed, enforce the same fmt+clippy gates as CI.
# Clippy style lints are NOT fixed by rustfmt — this catches them before push.
set -euo pipefail

input="$(cat)"
status="$(printf '%s' "$input" | node -e '
  let d = "";
  process.stdin.on("data", (c) => (d += c));
  process.stdin.on("end", () => {
    try {
      process.stdout.write(JSON.parse(d).status || "");
    } catch {
      process.stdout.write("");
    }
  });
')"

# Only auto-continue when the agent finished successfully.
if [[ "$status" != "completed" ]]; then
  echo '{}'
  exit 0
fi

# Skip if no Rust changes in the working tree (vs HEAD).
changed_rs="$(git diff --name-only HEAD 2>/dev/null; git diff --cached --name-only 2>/dev/null || true)"
if ! printf '%s\n' "$changed_rs" | grep -q '\.rs$'; then
  echo '{}'
  exit 0
fi

fail_msg=""

if ! cargo fmt --all -- --check >/dev/null 2>&1; then
  fail_msg+="cargo fmt --check failed. Run: cargo fmt --all"$'\n'
fi

clippy_out="$(mktemp)"
if ! cargo clippy --workspace --all-targets -- -D warnings >"$clippy_out" 2>&1; then
  clippy_errors="$(grep -E '^error:|--> ' "$clippy_out" | head -n 40 || true)"
  fail_msg+="cargo clippy -D warnings failed (same gate as GitHub Actions)."$'\n'
  fail_msg+="${clippy_errors}"$'\n'
  fail_msg+="Fix these Clippy lints (style/idiom — not test failures), then stop again."
fi
rm -f "$clippy_out"

if [[ -z "$fail_msg" ]]; then
  echo '{}'
  exit 0
fi

FOLLOWUP_MESSAGE="$fail_msg" node -e '
  process.stdout.write(JSON.stringify({
    followup_message: process.env.FOLLOWUP_MESSAGE || ""
  }));
'
exit 0
