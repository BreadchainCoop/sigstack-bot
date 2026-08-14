# Ops handoff: dual-CVM Phala deploy

Finish Signal registration + smoke test yourself. Everything below assumes workspace **Bread Coop** and images under **`daopunk/`**.

Related: [2026-08-05-dual-cvm-phala-deploy.md](./2026-08-05-dual-cvm-phala-deploy.md), [two-cvm-architecture.md](../two-cvm-architecture.md).

---

## Done

| Step | Status |
|------|--------|
| Fix `SIGNAL_PROXY_IMAGE` env name | Done ([`docker/phala.translation.env.example`](../../docker/phala.translation.env.example)) |
| Rewrite [`scripts/deploy_phala.sh`](../../scripts/deploy_phala.sh) for dual `tdx.medium` | Done |
| Gitignored Phala env files with phones / NEAR key / digests / ops SSH pubkey | Done (`docker/phala.transcription.env`, `docker/phala.translation.env`) |
| Build + push `linux/amd64` images | Done — `daopunk/signal-bot-tee`, `signal-whisper-api`, `signal-registration-proxy` |
| Deploy 2× `tdx.medium` (2 vCPU / 4 GB RAM, 40 GB disk) | Done |
| Digest-pin images in env + redeploy | Done |
| Registration proxy on **both** CVMs (`:8081`) | Done |
| Ops SSH via `~/.ssh/phala_sigstack` + `DSTACK_AUTHORIZED_KEYS` | Done |
| Unpause Phala section in [`docs/language-threads.md`](../language-threads.md) | Done |

### Live CVMs

| Name | Role | App ID | Dashboard |
|------|------|--------|-----------|
| `sigstack-transcription` | Whisper + phone A | `c5df154154e8c61105fefe84d43515e69b0e2537` | https://cloud.phala.com/dashboard/cvms/eba19afc-0c26-4409-b026-f757928d2ef8 |
| `sigstack-translation` | Hub + phone B | `9adac7636fe255182f699940ffd1924960415507` | https://cloud.phala.com/dashboard/cvms/0e82fa77-8b15-4dbd-89c4-9045ab911353 |

**Registration proxies (Phala gateway):**

```text
TX: https://c5df154154e8c61105fefe84d43515e69b0e2537-8081.dstack-pha-prod9.phala.network
TR: https://9adac7636fe255182f699940ffd1924960415507-8081.dstack-pha-prod9.phala.network
```

**SSH (ops key you created):**

```bash
phala ssh sigstack-transcription -- -i ~/.ssh/phala_sigstack -o IdentitiesOnly=yes
phala ssh sigstack-translation   -- -i ~/.ssh/phala_sigstack -o IdentitiesOnly=yes
```

**Quick health checks:**

```bash
phala cvms list
phala ps --cvm-id sigstack-transcription
phala ps --cvm-id sigstack-translation

curl -sfS https://c5df154154e8c61105fefe84d43515e69b0e2537-8081.dstack-pha-prod9.phala.network/health
curl -sfS https://9adac7636fe255182f699940ffd1924960415507-8081.dstack-pha-prod9.phala.network/health
```

Expect JSON `{"status":"ok",...,"signal_api_healthy":true}` and containers including `signal-api`, `signal-bot`, `signal-registration-proxy` (plus `whisper-api` on transcription).

---

## Left for you

### 1. Register both phones **on the CVMs** (not local Docker)

Local Compose already has the numbers registered. That session does **not** live on Phala. Each CVM’s `signal-api` still has `[]` accounts until you register there.

**Warning:** Re-registering on a CVM takes over the number from local Signal CLI. Stop local bots first if you care about a clean cutover:

```bash
docker compose -f docker/compose.transcription.yaml --env-file docker/transcription.env stop signal-bot
docker compose -f docker/compose.translation.yaml --env-file docker/translation.env stop signal-bot
```

**A. Captcha** — open https://signalcaptchas.org/registration/generate.html twice. Copy the full `signalcaptcha://…` string each time (tokens expire quickly).

**B. Register** (phones match your local env: TX `+573103479014`, TR `+573107677679`):

```bash
TX_PROXY=https://c5df154154e8c61105fefe84d43515e69b0e2537-8081.dstack-pha-prod9.phala.network
TR_PROXY=https://9adac7636fe255182f699940ffd1924960415507-8081.dstack-pha-prod9.phala.network
PHONE_TX=+573103479014
PHONE_TR=+573107677679

# Transcription (phone A)
curl -sS -X POST "$TX_PROXY/v1/register/${PHONE_TX}" \
  -H 'Content-Type: application/json' \
  -d '{"captcha":"signalcaptcha://PASTE_TX_TOKEN","use_voice":false}'

# Translation (phone B)
curl -sS -X POST "$TR_PROXY/v1/register/${PHONE_TR}" \
  -H 'Content-Type: application/json' \
  -d '{"captcha":"signalcaptcha://PASTE_TR_TOKEN","use_voice":false}'
```

Helper script (same URLs/phones baked in): [`scripts/register_phala_phones.sh`](../../scripts/register_phala_phones.sh)

```bash
CAPTCHA_TX='signalcaptcha://...' CAPTCHA_TR='signalcaptcha://...' ./scripts/register_phala_phones.sh
```

**C. Verify SMS codes:**

```bash
curl -sS -X POST "$TX_PROXY/v1/register/${PHONE_TX}/verify/XXXXXX" \
  -H 'Content-Type: application/json' -d '{}'

curl -sS -X POST "$TR_PROXY/v1/register/${PHONE_TR}/verify/YYYYYY" \
  -H 'Content-Type: application/json' -d '{}'
```

Or:

```bash
SMS_TX=XXXXXX SMS_TR=YYYYYY ./scripts/register_phala_phones.sh --verify
```

**D. Confirm accounts, then restart bots on the CVMs:**

```bash
curl -sS "$TX_PROXY/v1/debug/signal-accounts"
curl -sS "$TR_PROXY/v1/debug/signal-accounts"
# Expect each CLI to list its phone.

phala ssh sigstack-transcription -- -i ~/.ssh/phala_sigstack -o IdentitiesOnly=yes \
  'docker restart dstack-signal-bot-1'

phala ssh sigstack-translation -- -i ~/.ssh/phala_sigstack -o IdentitiesOnly=yes \
  'docker restart dstack-signal-bot-1'
```

### 2. Smoke test in Signal

1. Create (or use) a test group; add **both** bot numbers.
2. Hub (translation): `!help`, `!info`, `!privacy`.
3. Pairing: `!transcription` from the hub; confirm the transcription bot joins / responds.
4. Short voice note → transcription posts text.
5. `!verify <challenge>` on each bot (separate CVM quotes).
6. If stuck: `phala logs --cvm-id sigstack-transcription` / `sigstack-translation`.

### 3. Optional cleanup

- Local dual Compose: leave stopped, or `down` (do **not** `down -v` unless you intend to wipe local Signal state).
- Old stopped CVM `dstack-app-hqvaf` was already gone from `phala cvms list` earlier; confirm with `phala cvms list`.

---

## Useful paths

| Path | Purpose |
|------|---------|
| `docker/phala.transcription.yaml` / `.env` | Transcription CVM compose + secrets |
| `docker/phala.translation.yaml` / `.env` | Translation CVM compose + secrets |
| `scripts/deploy_phala.sh` | Redeploy both CVMs |
| `scripts/register_phala_phones.sh` | Register/verify via gateways |
| `~/.ssh/phala_sigstack` | Passphrase-free ops SSH key |

Never commit `docker/phala.*.env` (gitignored).
