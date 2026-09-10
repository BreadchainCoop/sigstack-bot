#!/bin/bash
# Register SIGNAL_PHONE from docker/.phala.env via the live CVM proxy :8081.
#
# Skip if /v1/debug/signal-accounts already lists that number.
# Do not point this at local docker/.env — that is a different number.
#
# Usage:
#   CAPTCHA_TR='signalcaptcha://...' ./scripts/register_phala_phone.sh
# Then when SMS arrives:
#   SMS_TR=123456 ./scripts/register_phala_phone.sh --verify
#
# Optional overrides: PHONE_TR, APP_ID, TR_PROXY, ENV_FILE
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ENV_FILE="${ENV_FILE:-$ROOT/docker/.phala.env}"

if [[ ! -f "$ENV_FILE" ]]; then
  echo "Error: missing $ENV_FILE (copy docker/.phala.env.example)" >&2
  exit 1
fi

# Load SIGNAL_PHONE from Phala env without sourcing the whole file.
SIGNAL_PHONE_FROM_ENV="$(
  grep -E '^SIGNAL_PHONE=' "$ENV_FILE" | head -n1 | cut -d= -f2- | tr -d '\r' | tr -d '"' | tr -d "'"
)"
if [[ -z "${SIGNAL_PHONE_FROM_ENV}" ]]; then
  echo "Error: SIGNAL_PHONE unset in $ENV_FILE" >&2
  exit 1
fi

APP_ID="${APP_ID:-9adac7636fe255182f699940ffd1924960415507}"
GATEWAY="${GATEWAY:-dstack-pha-prod9.phala.network}"
TR_PROXY="${TR_PROXY:-https://${APP_ID}-8081.${GATEWAY}}"
# Prefer explicit PHONE_TR; otherwise always use docker/.phala.env SIGNAL_PHONE.
PHONE_TR="${PHONE_TR:-$SIGNAL_PHONE_FROM_ENV}"

MODE="${1:-register}"

if [[ "$PHONE_TR" == "$SIGNAL_PHONE_FROM_ENV" ]]; then
  echo "Using phone $PHONE_TR from $ENV_FILE"
else
  echo "Using phone $PHONE_TR (PHONE_TR override; .phala.env has $SIGNAL_PHONE_FROM_ENV)"
fi
echo "Proxy: $TR_PROXY"

if [[ "$MODE" == "--verify" ]]; then
  if [[ -z "${SMS_TR:-}" ]]; then
    echo "Error: set SMS_TR" >&2
    exit 1
  fi
  echo "Verifying $PHONE_TR on :8081..."
  curl -sS -X POST "$TR_PROXY/v1/register/${PHONE_TR}/verify/${SMS_TR}" \
    -H 'Content-Type: application/json' -d '{}'
  echo
else
  if [[ -z "${CAPTCHA_TR:-}" ]]; then
    echo "Error: set CAPTCHA_TR (full signalcaptcha://… string)" >&2
    exit 1
  fi
  echo "Registering $PHONE_TR on :8081..."
  curl -sS -X POST "$TR_PROXY/v1/register/${PHONE_TR}" \
    -H 'Content-Type: application/json' \
    -d "$(python3 -c 'import json,os; print(json.dumps({"captcha":os.environ["CAPTCHA_TR"],"use_voice":False}))')"
  echo
fi

echo "Accounts (CVM :8081):"
curl -sS "$TR_PROXY/v1/debug/signal-accounts"; echo
