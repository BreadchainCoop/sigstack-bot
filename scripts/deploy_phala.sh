#!/bin/bash
# Deploy the one-CVM product suite (one Signal number, no local Whisper).
#
# Live upgrades MUST use --cvm-id so volumes stay (phone B + group prefs):
#   CVM_ID=0e82fa77-8b15-4dbd-89c4-9045ab911353 ./scripts/deploy_phala.sh
#
# Do NOT `phala deploy -n` against the live translation CVM — that can create a
# replacement with empty volumes (lost Signal session + user prefs).
# Do NOT deploy docker/phala.transcription.yaml (deprecated stub).
# See docs/two-cvm-architecture.md#cvm-storage-keep-intact
#
# First create (empty volumes) only when no CVM exists:
#   FIRST_CREATE=1 ./scripts/deploy_phala.sh
#
# Requires: phala CLI logged in; filled docker/phala.translation.env (never commit).
# Images must already be pushed (linux/amd64).
set -euo pipefail

if ! command -v phala &> /dev/null; then
  echo "Error: phala CLI not found. Install with: npm install -g phala"
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TR_COMPOSE="${TR_COMPOSE:-$ROOT/docker/phala.translation.yaml}"
TR_ENV="${TR_ENV:-$ROOT/docker/phala.translation.env}"
TR_NAME="${TR_NAME:-sigstack-translation}"
INSTANCE_TYPE="${INSTANCE_TYPE:-tdx.medium}"
DISK_SIZE="${DISK_SIZE:-40G}"
# Surviving prod CVM (phone B).
CVM_ID="${CVM_ID:-0e82fa77-8b15-4dbd-89c4-9045ab911353}"
FIRST_CREATE="${FIRST_CREATE:-0}"

if [[ ! -f "$TR_COMPOSE" ]]; then
  echo "Error: missing $TR_COMPOSE"
  exit 1
fi
if [[ ! -f "$TR_ENV" ]]; then
  echo "Error: missing $TR_ENV (copy docker/phala.translation.env.example)"
  exit 1
fi

if [[ "$FIRST_CREATE" == "1" ]]; then
  echo "First-create $TR_NAME ($INSTANCE_TYPE, disk $DISK_SIZE) — empty volumes; register phone after."
  phala deploy \
    -n "$TR_NAME" \
    -c "$TR_COMPOSE" \
    -e "$TR_ENV" \
    -t "$INSTANCE_TYPE" \
    --disk-size "$DISK_SIZE" \
    --wait
else
  echo "In-place upgrade of CVM $CVM_ID ($TR_COMPOSE)..."
  phala deploy \
    --cvm-id "$CVM_ID" \
    -c "$TR_COMPOSE" \
    -e "$TR_ENV" \
    --wait
fi

echo ""
echo "Done. Check status with: phala cvms list"
echo "Phone B stays on this CVM (proxy :8081). Do not re-register phone A."
