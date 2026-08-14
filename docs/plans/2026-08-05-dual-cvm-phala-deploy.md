# Plan: Dual-CVM Phala deploy (2× tdx.medium)

**Goal:** Deploy the product suite on two Phala TDX CVMs — **2 vCPU / 4 GB each** (`tdx.medium`) — matching [`docs/two-cvm-architecture.md`](../two-cvm-architecture.md).

| CVM | Instance | Compose | Role | Live |
|-----|----------|---------|------|------|
| `sigstack-transcription` | `tdx.medium` | [`docker/phala.transcription.yaml`](../../docker/phala.transcription.yaml) | Phone A + Whisper + `BOT__ROLE=transcription` | running — app `c5df154154e8c61105fefe84d43515e69b0e2537` |
| `sigstack-translation` | `tdx.medium` | [`docker/phala.translation.yaml`](../../docker/phala.translation.yaml) | Phone B + NEAR AI + `BOT__ROLE=translation` + registration proxy | running — app `9adac7636fe255182f699940ffd1924960415507` |

**Cost:** ~$0.232/hr combined (~$167/mo if always on).  
**Auth:** Phala CLI / workspace **Bread Coop**. Images digest-pinned in gitignored `docker/phala.*.env`.

**Registration proxies:**
- TX: `https://c5df154154e8c61105fefe84d43515e69b0e2537-8081.dstack-pha-prod9.phala.network`
- TR: `https://9adac7636fe255182f699940ffd1924960415507-8081.dstack-pha-prod9.phala.network`

---

## Preflight (fix before first deploy)

1. **Env var name mismatch (blocker for translation proxy):**  
   `docker/phala.translation.yaml` uses `${SIGNAL_PROXY_IMAGE}`;  
   `docker/phala.translation.env.example` documents `SIGNAL_REGISTRATION_PROXY_IMAGE`.  
   Align to one name (prefer `SIGNAL_PROXY_IMAGE` to match compose + `deploy/dstack-app-ctqs1`).

2. **Refresh `scripts/deploy_phala.sh`:** still uses deprecated `phala cvms create` + single `phala-compose.yaml`. Either replace with two `phala deploy … -t tdx.medium` calls or document CLI-only and delete the script later.

3. **Images:** bot / whisper / proxy currently pulled by tag (`:latest`). Signal CLI is already digest-pinned. Optional-but-recommended: push and pin digests in compose for attestation (open follow-up in language-threads.md).

4. **Prerequisites to have on hand:**
   - Two Signal-capable E.164 numbers (A = transcription, B = translation)
   - Docker registry (Docker Hub or GHCR) with push access; images must be `linux/amd64`
   - `NEAR_AI_API_KEY` for translation CVM
   - Optional SSH pubkey if you want `--dev-os` / SSH into CVMs

---

## Phase 0 — Secrets + env files (local, never commit)

```bash
cp docker/phala.transcription.env.example docker/phala.transcription.env
cp docker/phala.translation.env.example docker/phala.translation.env
```

Fill:

| File | Required |
|------|----------|
| `phala.transcription.env` | `SIGNAL_PHONE`=A, `PEER_PHONE`=B, `SIGNAL_BOT_IMAGE`, `WHISPER_IMAGE` |
| `phala.translation.env` | `SIGNAL_PHONE`=B, `PEER_PHONE`=A, `SIGNAL_BOT_IMAGE`, `SIGNAL_PROXY_IMAGE`, `NEAR_AI_API_KEY` |

Confirm both env files are gitignored.

---

## Phase 1 — Build & push images (`linux/amd64`)

Replace `YOUR_DOCKERHUB` with the chosen org/user:

```bash
docker buildx build --platform linux/amd64 \
  -t YOUR_DOCKERHUB/signal-bot-tee:latest \
  -f docker/Dockerfile --push .

docker buildx build --platform linux/amd64 \
  -t YOUR_DOCKERHUB/signal-whisper-api:latest \
  -f docker/Dockerfile.whisper --push .

docker buildx build --platform linux/amd64 \
  -t YOUR_DOCKERHUB/signal-registration-proxy:latest \
  -f docker/Dockerfile.proxy --push .
```

If the registry is private, set Phala pull creds (`DSTACK_DOCKER_USERNAME` / `DSTACK_DOCKER_PASSWORD` or current Phala private-registry flow). Prefer public images for the first bring-up to reduce moving parts.

**Optional:** after push, `docker buildx imagetools inspect …` and pin `@sha256:…` in both Phala compose files.

---

## Phase 2 — Create the two CVMs

Prefer **new** CVMs (clean volumes). Do not co-locate Whisper with translation.

```bash
# Transcription worker
phala deploy \
  -n sigstack-transcription \
  -c docker/phala.transcription.yaml \
  -e docker/phala.transcription.env \
  -t tdx.medium \          # 2 vCPU / 4 GB RAM
  --disk-size 40G \        # 40 GB disk (not RAM); Phala default
  --wait

# Translation hub
phala deploy \
  -n sigstack-translation \
  -c docker/phala.translation.yaml \
  -e docker/phala.translation.env \
  -t tdx.medium \          # 2 vCPU / 4 GB RAM
  --disk-size 40G \        # 40 GB disk (not RAM)
  --wait
```

Verify:

```bash
phala cvms list
phala cvms get --cvm-id sigstack-transcription   # expect 2 vCPU / 4 GB
phala cvms get --cvm-id sigstack-translation
phala ps --cvm-id sigstack-transcription          # signal-api, whisper-api, signal-bot
phala ps --cvm-id sigstack-translation            # signal-api, signal-bot, signal-registration-proxy
```

Whisper on transcription may take several minutes for first model pull (`start_period: 300s`).

Cleanup (optional): `phala cvms delete --cvm-id dstack-app-hqvaf` once the new pair is healthy.

---

## Phase 3 — Register Signal phones on the CVMs

**Why:** Fresh CVMs have empty Signal CLI volumes. Registration that exists on your laptop Compose stacks does **not** transfer. Both numbers must be registered into each product CVM’s `signal-api` so plaintext Signal sessions live inside that TEE.

**Target = Phala CVMs, not local Docker.** Do not use `localhost:8080` / `localhost:8081` from the dual Compose guide for this step.

1. Captcha: https://signalcaptchas.org/registration/generate.html — copy the full `signalcaptcha://…` token.
2. **Translation CVM (phone B):** register + verify via that CVM’s registration proxy (`:8081` through Phala gateway / `phala ssh`) or its in-CVM `signal-api` `/v1/register/...`.
3. **Transcription CVM (phone A):** register + verify against **that** CVM’s `signal-api` only (no proxy in transcription compose — `phala ssh` / container exec).
4. Confirm `/v1/accounts` on each CVM; restart that CVM’s `signal-bot` if the session was created after the bot started.

Do **not** wipe CVM volumes after registration unless you intend to re-register.

---

## Phase 4 — Smoke test in a shared Signal group

1. Create a test group; add **both** bot numbers.
2. Hub (translation): `!help`, `!info`, `!privacy` — expect menus.
3. Pairing: from translation, `!transcription` invite path; confirm transcription auto-accepts / joins (PEER_PHONE wired both ways).
4. Voice: send a short voice note → transcription posts text; translation can act on that text if in-chat/threads enabled.
5. Attestation: `!verify <challenge>` on each bot (separate CVM quotes).
6. Logs: `phala logs --cvm-id …` if anything stalls.

---

## Phase 5 — Harden (same session or immediate follow-up)

- Pin bot/whisper/proxy digests in Phala compose; redeploy with `--wait`.
- Update `scripts/deploy_phala.sh` (or remove) so it matches dual `phala deploy -t tdx.medium`.
- Flip language-threads “Phala / TEE (paused)” section to live once verified.
- Document CVM names/IDs in an ops note (not secrets) for the Bread Coop workspace.

---

## Out of scope

- Stripe / multi-tenant “create your bot” website
- Reintroducing tools, x402, or general chat
- Cross-CVM Docker networking (Signal remains the only bus)
- Growing past `tdx.medium` unless Whisper OOMs (then resize transcription only)

---

## Decision checklist (before executing)

- [ ] Registry org/name chosen and push works for `linux/amd64`
- [ ] Phone A + Phone B ready (or known acquisition path)
- [ ] `NEAR_AI_API_KEY` available
- [ ] Fix `SIGNAL_PROXY_IMAGE` env name
- [ ] Delete vs ignore stopped `dstack-app-hqvaf`
- [ ] Go / no-go on digest pinning in the first deploy vs phase 5
