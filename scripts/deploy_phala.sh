#!/bin/bash
# Deploy the one-CVM product suite (one Signal number, no local Whisper).
#
# Live upgrades MUST use --cvm-id so volumes stay (registered phone + group prefs):
#   CVM_ID=0e82fa77-8b15-4dbd-89c4-9045ab911353 ./scripts/deploy_phala.sh
#
# Do NOT `phala deploy -n` against the live CVM — that can create a
# replacement with empty volumes (lost Signal session + user prefs).
# See docs/one-cvm-architecture.md#cvm-storage-keep-intact
#
# First create (empty volumes) only when no CVM exists:
#   FIRST_CREATE=1 ./scripts/deploy_phala.sh
#
# Requires: phala CLI logged in; filled docker/phala.env (never commit).
# Images must already be pushed (linux/amd64).
set -euo pipefail

if ! command -v phala &> /dev/null; then
  echo "Error: phala CLI not found. Install with: npm install -g phala"
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
COMPOSE="${COMPOSE:-$ROOT/docker/phala.yaml}"
ENV_FILE="${ENV_FILE:-$ROOT/docker/phala.env}"
NAME="${NAME:-sigstack-translation}"
INSTANCE_TYPE="${INSTANCE_TYPE:-tdx.medium}"
DISK_SIZE="${DISK_SIZE:-40G}"
# Surviving prod CVM.
CVM_ID="${CVM_ID:-0e82fa77-8b15-4dbd-89c4-9045ab911353}"
FIRST_CREATE="${FIRST_CREATE:-0}"

if [[ ! -f "$COMPOSE" ]]; then
  echo "Error: missing $COMPOSE"
  exit 1
fi
if [[ ! -f "$ENV_FILE" ]]; then
  echo "Error: missing $ENV_FILE (copy docker/phala.env.example)"
  exit 1
fi

if [[ "$FIRST_CREATE" == "1" ]]; then
  echo "First-create $NAME ($INSTANCE_TYPE, disk $DISK_SIZE) — empty volumes; register phone after."
  phala deploy \
    -n "$NAME" \
    -c "$COMPOSE" \
    -e "$ENV_FILE" \
    -t "$INSTANCE_TYPE" \
    --disk-size "$DISK_SIZE" \
    --wait
else
  echo "In-place upgrade of CVM $CVM_ID ($COMPOSE)..."
  phala deploy \
    --cvm-id "$CVM_ID" \
    -c "$COMPOSE" \
    -e "$ENV_FILE" \
    --wait
fi

echo ""
echo "Done. Check status with: phala cvms list"
echo "The registered number stays on this CVM (proxy :8081)."
