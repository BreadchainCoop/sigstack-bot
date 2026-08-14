#!/bin/bash
# Register Signal phones onto the live Phala CVMs via each CVM's registration proxy.
# Usage:
#   CAPTCHA_TX='signalcaptcha://...' CAPTCHA_TR='signalcaptcha://...' ./scripts/register_phala_phones.sh
# Then when SMS arrives:
#   SMS_TX=123456 SMS_TR=654321 ./scripts/register_phala_phones.sh --verify
set -euo pipefail

TX_PROXY="${TX_PROXY:-https://c5df154154e8c61105fefe84d43515e69b0e2537-8081.dstack-pha-prod9.phala.network}"
TR_PROXY="${TR_PROXY:-https://9adac7636fe255182f699940ffd1924960415507-8081.dstack-pha-prod9.phala.network}"
PHONE_TX="${PHONE_TX:-+573103479014}"
PHONE_TR="${PHONE_TR:-+573107677679}"

MODE="${1:-register}"

if [[ "$MODE" == "--verify" ]]; then
  : "${SMS_TX:?set SMS_TX}"
  : "${SMS_TR:?set SMS_TR}"
  echo "Verifying transcription $PHONE_TX..."
  curl -sS -X POST "$TX_PROXY/v1/register/${PHONE_TX}/verify/${SMS_TX}" \
    -H 'Content-Type: application/json' -d '{}'
  echo
  echo "Verifying translation $PHONE_TR..."
  curl -sS -X POST "$TR_PROXY/v1/register/${PHONE_TR}/verify/${SMS_TR}" \
    -H 'Content-Type: application/json' -d '{}'
  echo
else
  : "${CAPTCHA_TX:?set CAPTCHA_TX to full signalcaptcha:// token}"
  : "${CAPTCHA_TR:?set CAPTCHA_TR to full signalcaptcha:// token}"
  echo "Registering transcription $PHONE_TX on CVM proxy..."
  curl -sS -X POST "$TX_PROXY/v1/register/${PHONE_TX}" \
    -H 'Content-Type: application/json' \
    -d "$(python3 -c 'import json,os; print(json.dumps({"captcha":os.environ["CAPTCHA_TX"],"use_voice":False}))')"
  echo
  echo "Registering translation $PHONE_TR on CVM proxy..."
  curl -sS -X POST "$TR_PROXY/v1/register/${PHONE_TR}" \
    -H 'Content-Type: application/json' \
    -d "$(python3 -c 'import json,os; print(json.dumps({"captcha":os.environ["CAPTCHA_TR"],"use_voice":False}))')"
  echo
fi

echo "Accounts:"
curl -sS "$TX_PROXY/v1/debug/signal-accounts"; echo
curl -sS "$TR_PROXY/v1/debug/signal-accounts"; echo
