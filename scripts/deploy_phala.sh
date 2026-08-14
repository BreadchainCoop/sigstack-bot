#!/bin/bash
# First-create both product CVMs on Phala Cloud (2× tdx.medium = 2 vCPU / 4 GB RAM each).
# Uses `phala deploy -n` (new names). Do NOT run this against live registered CVMs —
# that would risk empty volumes (lost Signal sessions + user prefs). Upgrade existing
# CVMs with `phala deploy --cvm-id <existing>` so signal-config-* and group-prefs-* stay.
# See docs/two-cvm-architecture.md#cvm-storage-keep-intact and AGENTS.md.
# Requires: phala CLI logged in; filled docker/phala.transcription.env + docker/phala.translation.env
# (never commit those env files). Images must already be pushed (linux/amd64).
set -euo pipefail

if ! command -v phala &> /dev/null; then
  echo "Error: phala CLI not found. Install with: npm install -g phala"
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TX_COMPOSE="${TX_COMPOSE:-$ROOT/docker/phala.transcription.yaml}"
TR_COMPOSE="${TR_COMPOSE:-$ROOT/docker/phala.translation.yaml}"
TX_ENV="${TX_ENV:-$ROOT/docker/phala.transcription.env}"
TR_ENV="${TR_ENV:-$ROOT/docker/phala.translation.env}"
TX_NAME="${TX_NAME:-sigstack-transcription}"
TR_NAME="${TR_NAME:-sigstack-translation}"
INSTANCE_TYPE="${INSTANCE_TYPE:-tdx.medium}"
DISK_SIZE="${DISK_SIZE:-40G}"

for f in "$TX_COMPOSE" "$TR_COMPOSE" "$TX_ENV" "$TR_ENV"; do
  if [[ ! -f "$f" ]]; then
    echo "Error: missing $f"
    exit 1
  fi
done

echo "Deploying $TX_NAME ($INSTANCE_TYPE, disk $DISK_SIZE)..."
phala deploy \
  -n "$TX_NAME" \
  -c "$TX_COMPOSE" \
  -e "$TX_ENV" \
  -t "$INSTANCE_TYPE" \
  --disk-size "$DISK_SIZE" \
  --wait

echo "Deploying $TR_NAME ($INSTANCE_TYPE, disk $DISK_SIZE)..."
phala deploy \
  -n "$TR_NAME" \
  -c "$TR_COMPOSE" \
  -e "$TR_ENV" \
  -t "$INSTANCE_TYPE" \
  --disk-size "$DISK_SIZE" \
  --wait

echo ""
echo "Both CVMs deployed. Check status with: phala cvms list"
