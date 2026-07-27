#!/usr/bin/env bash
# Auto-format Rust files after agent/tab edits (rustfmt — same as CI Format step).
set -euo pipefail

input="$(cat)"
file_path="$(printf '%s' "$input" | node -e '
  let d = "";
  process.stdin.on("data", (c) => (d += c));
  process.stdin.on("end", () => {
    try {
      const j = JSON.parse(d);
      process.stdout.write(j.file_path || "");
    } catch {
      process.stdout.write("");
    }
  });
')"

if [[ -z "$file_path" || "$file_path" != *.rs ]]; then
  exit 0
fi

if ! command -v rustfmt >/dev/null 2>&1 && ! command -v cargo >/dev/null 2>&1; then
  exit 0
fi

# Format the edited file in place (matches `cargo fmt` style).
cargo fmt -- "$file_path" >/dev/null 2>&1 || true
exit 0
