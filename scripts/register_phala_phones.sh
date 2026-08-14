#!/bin/bash
# Register Signal phones onto the live one-CVM Phala stack via registration proxies.
#
# After the transcription CVM was deleted, phone A must be re-registered on :8082
# of the surviving translation CVM. Phone B on :8081 should already be registered
# — skip CAPTCHA_TR / SMS_TR if accounts already lists it.
#
# Usage:
#   CAPTCHA_TX='signalcaptcha://...' ./scripts/register_phala_phones.sh
#   CAPTCHA_TX='...' CAPTCHA_TR='...' ./scripts/register_phala_phones.sh   # both
# Then when SMS arrives:
#   SMS_TX=123456 ./scripts/register_phala_phones.sh --verify
#   SMS_TX=123456 SMS_TR=654321 ./scripts/register_phala_phones.sh --verify
set -euo pipefail

APP_ID="${APP_ID:-9adac7636fe255182f699940ffd1924960415507}"
GATEWAY="${GATEWAY:-dstack-pha-prod9.phala.network}"
# Transcription (phone A) proxy on the surviving CVM.
TX_PROXY="${TX_PROXY:-https://${APP_ID}-8082.${GATEWAY}}"
# Translation (phone B) proxy — already registered; keep for verify/debug.
TR_PROXY="${TR_PROXY:-https://${APP_ID}-8081.${GATEWAY}}"
PHONE_TX="${PHONE_TX:-+573103479014}"
PHONE_TR="${PHONE_TR:-+573107677679}"

MODE="${1:-register}"

if [[ "$MODE" == "--verify" ]]; then
  if [[ -n "${SMS_TX:-}" ]]; then
    echo "Verifying transcription $PHONE_TX on :8082..."
    curl -sS -X POST "$TX_PROXY/v1/register/${PHONE_TX}/verify/${SMS_TX}" \
      -H 'Content-Type: application/json' -d '{}'
    echo
  fi
  if [[ -n "${SMS_TR:-}" ]]; then
    echo "Verifying translation $PHONE_TR on :8081..."
    curl -sS -X POST "$TR_PROXY/v1/register/${PHONE_TR}/verify/${SMS_TR}" \
      -H 'Content-Type: application/json' -d '{}'
    echo
  fi
  if [[ -z "${SMS_TX:-}" && -z "${SMS_TR:-}" ]]; then
    echo "Error: set SMS_TX and/or SMS_TR" >&2
    exit 1
  fi
else
  if [[ -z "${CAPTCHA_TX:-}" && -z "${CAPTCHA_TR:-}" ]]; then
    echo "Error: set CAPTCHA_TX (phone A / :8082) and/or CAPTCHA_TR (phone B / :8081)" >&2
    exit 1
  fi
  if [[ -n "${CAPTCHA_TX:-}" ]]; then
    echo "Registering transcription $PHONE_TX on :8082..."
    curl -sS -X POST "$TX_PROXY/v1/register/${PHONE_TX}" \
      -H 'Content-Type: application/json' \
      -d "$(python3 -c 'import json,os; print(json.dumps({"captcha":os.environ["CAPTCHA_TX"],"use_voice":False}))')"
    echo
  fi
  if [[ -n "${CAPTCHA_TR:-}" ]]; then
    echo "Registering translation $PHONE_TR on :8081..."
    curl -sS -X POST "$TR_PROXY/v1/register/${PHONE_TR}" \
      -H 'Content-Type: application/json' \
      -d "$(python3 -c 'import json,os; print(json.dumps({"captcha":os.environ["CAPTCHA_TR"],"use_voice":False}))')"
    echo
  fi
fi

echo "Accounts (translation :8081 / transcription :8082):"
curl -sS "$TR_PROXY/v1/debug/signal-accounts"; echo
curl -sS "$TX_PROXY/v1/debug/signal-accounts"; echo
