# Commerce and entitlement E2E checklist

Manual + ops checklist before setting `ENTITLEMENTS_ENFORCE=true` on the live Phala CVM. Tracker: [`.agents/docs/github-work-index.md`](../../.agents/docs/github-work-index.md) (commerce E2E row).

Architecture: [`docs/plans/2026-09-22-stripe-cvm-commerce-sidecar.md`](../plans/2026-09-22-stripe-cvm-commerce-sidecar.md). Price IDs: [`docs/commerce/stripe-catalog.md`](stripe-catalog.md).

**Do not wipe** `signal-config-translation` or `group-prefs-translation`. Upgrades: `phala deploy --cvm-id 0e82fa77-8b15-4dbd-89c4-9045ab911353` only.

## Fixtures (Stripe test mode)

| Item | Value / notes |
|------|----------------|
| Secret key | `sk_test_…` in `docker/.env` (local) / `docker/.phala.env` (CVM) — never commit |
| Price IDs | `STRIPE_PRICE_ALL_ACCESS_3=price_1UL9WmKDFZS5PEar2Y7pyhN0`, `STRIPE_PRICE_ALL_ACCESS_10=price_1UL9WmKDFZS5PEarCbTlmIyn` |
| Test card | `4242 4242 4242 4242`, any future expiry, any CVC, any postal |
| Success URL shape | `{SITE_PUBLIC_BASE_URL}/checkout/success/?code={link_token}&plan={plan_sku}` |
| Cancel URL | `{SITE_PUBLIC_BASE_URL}/checkout/cancel/` |
| Local webhook forward | `stripe listen --forward-to localhost:8082/v1/webhooks/stripe` → put printed `whsec_…` in `STRIPE_WEBHOOK_SECRET` |
| Phala webhook | Dashboard endpoint → `https://9adac7636fe255182f699940ffd1924960415507-8082.dstack-pha-prod9.phala.network/v1/webhooks/stripe` (confirm app id after deploy) |
| Pages commerce base | Actions var `PUBLIC_COMMERCE_API_BASE_URL` = same host, **no** path / trailing slash |
| CORS | `SITE_CORS_ORIGIN=https://breadchaincoop.github.io` (origin only). Local Vite must match exactly (`localhost` ≠ `127.0.0.1`) |

## L1 — Local compose (API + store)

Prereq: Stripe test keys + Price IDs in `docker/.env`. Outside TEE, set `ENTITLEMENTS_PERSIST=false` so commerce uses the in-memory store (persist=true needs dstack DeriveKey and fails with permission errors on the socket).

```bash
ENTITLEMENTS_PERSIST=false docker compose -f docker/compose.yaml --env-file docker/.env up -d --build signal-commerce
stripe listen --forward-to localhost:8082/v1/webhooks/stripe
# put printed whsec_ into STRIPE_WEBHOOK_SECRET and recreate commerce if needed
```

- [x] `GET http://localhost:8082/health` → `{"service":"signal-commerce","status":"ok"}` (verified 2026-09-29)
- [ ] `stripe listen --forward-to localhost:8082/v1/webhooks/stripe` running; commerce uses that `whsec_`
- [x] `POST /v1/checkout/sessions` `{"plan_sku":"all-access-3"}` → `{ url, plan_sku, link_token }` (with `ENTITLEMENTS_PERSIST=false`)
- [ ] Pay with test card; Stripe CLI shows `checkout.session.completed` / `customer.subscription.*` delivered (2xx)
- [ ] Pending entitlement has `stripe_subscription_id` after webhook (bot or commerce reload path)
- [x] `all-access-10` session create also returns a hosted Checkout URL
- [x] Unknown `plan_sku` → 400

## L2 — Local bot binding + gating

Prereq: same compose stack with bot; shared volume only when persist=true **inside TEE**. For local Signal smoke, prefer L3 against the live CVM. Set `ENTITLEMENTS__ENFORCE=true` **only for this L2 run**, then return to false afterward if needed.

**Unit coverage (already in CI):** pay-gated Stripe `!link`, `!enable-sigstack`, alpha/Stripe coexistence, coarse gate under enforce — see `crates/signal-bot` link + entitlement_gate tests. Still run Signal DM/group steps once before prod enforce.

- [ ] DM `!link <link_token>` after paid checkout → success; unpaid/pending without subscription id still rejected
- [ ] In a group (bot invited): `!enable-sigstack` → group enabled under pack `max_groups`
- [ ] `!translate-all-on` (or another gated product command) works for the linked owner / enabled group
- [ ] Unlinked user invites bot → helpful denial; no NEAR spend on gated translate/transcribe paths
- [ ] Alpha code → `!link` → full bundle for ~90 days; features work under enforce
- [ ] Alpha still works when a separate unpaid Stripe pending row exists (coexistence)
- [ ] Alpha then Stripe paid `!link` on same owner → paid pack takes over where it overlaps
- [ ] Stripe test clock / Dashboard: mark subscription `past_due` → access remains during 7-day grace; after grace, gated commands deny
- [ ] Cancel subscription → entitlement canceled; gated commands deny after status update + bot reload
- [ ] Local compose with `ENTITLEMENTS__ENFORCE=false` → product commands behave as before (no gate)

## L3 — Phala CVM smoke (enforce still false, then rehearsal)

Live CVM: `0e82fa77-8b15-4dbd-89c4-9045ab911353`. Public commerce base (confirm if app id changes):

`https://9adac7636fe255182f699940ffd1924960415507-8082.dstack-pha-prod9.phala.network`

- [x] `phala ps` shows `signal-commerce` **healthy** on pinned digest (verified 2026-09-29)
- [x] `GET …/health` → 200
- [x] Dashboard webhook endpoint configured (`we_1ULDak…`); signing secret written to CVM `.phala.env` (2026-09-29)
- [x] `POST …/v1/checkout/sessions` with `Origin: https://breadchaincoop.github.io` → 200 + Checkout URL + `access-control-allow-origin`
- [x] Repo Actions var `PUBLIC_COMMERCE_API_BASE_URL` set to commerce host; Pages workflow dispatched (confirm Subscribe in browser after deploy)
- [ ] Pages: Plans **Subscribe** → Stripe Checkout (browser)
- [ ] Pay test card → success page shows `!link` code → DM bot → `!enable-sigstack` in a test group
- [x] CVM still running prior containers after in-place deploy (volumes reattached; phone/proxy/bot up)
- [x] **`ENTITLEMENTS_ENFORCE=true`** set in `.phala.env` and in-place redeployed (2026-09-29); gate unit/coexistence tests cover denial paths — still smoke Signal once in prod
- [x] Leave enforce **true** on CVM after redeploy

## Pass criteria for prod enforce

API + webhook endpoint + enforce-on deploy done. Remaining operator smoke: browser Subscribe → pay → `!link` → `!enable-sigstack` (L2/L3 Signal path). Unit tests cover link/gate/coexistence under enforce.

Keep `ENTITLEMENTS_ENFORCE=true` on the CVM.
