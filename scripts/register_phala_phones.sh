#!/bin/bash
# Register the surviving Signal number (phone B) via proxy :8081.
#
# Phone B should already be registered on the live CVM — skip if
# accounts already lists it. Do not re-register phone A.
#
# Usage:
#   CAPTCHA_TR='signalcaptcha://...' ./scripts/register_phala_phones.sh
# Then when SMS arrives:
#   SMS_TR=123456 ./scripts/register_phala_phones.sh --verify
set -euo pipefail

APP_ID="${APP_ID:-9adac7636fe255182f699940ffd1924960415507}"
GATEWAY="${GATEWAY:-dstack-pha-prod9.phala.network}"
TR_PROXY="${TR_PROXY:-https://${APP_ID}-8081.${GATEWAY}}"
PHONE_TR="${PHONE_TR:-+573107677679}"

MODE="${1:-register}"

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
    echo "Error: set CAPTCHA_TR (phone B / :8081)" >&2
    exit 1
  fi
  echo "Registering $PHONE_TR on :8081..."
  curl -sS -X POST "$TR_PROXY/v1/register/${PHONE_TR}" \
    -H 'Content-Type: application/json' \
    -d "$(python3 -c 'import json,os; print(json.dumps({"captcha":os.environ["CAPTCHA_TR"],"use_voice":False}))')"
  echo
fi

echo "Accounts (translation :8081):"
curl -sS "$TR_PROXY/v1/debug/signal-accounts"; echo
