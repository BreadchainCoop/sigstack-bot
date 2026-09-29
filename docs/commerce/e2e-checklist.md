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

Prereq: `docker compose -f docker/compose.yaml --env-file docker/.env up -d` with Stripe test keys + Price IDs; `ENTITLEMENTS__ENFORCE` can stay false.

- [ ] `GET http://localhost:8082/health` → `{"service":"signal-commerce","status":"ok"}`
- [ ] `stripe listen --forward-to localhost:8082/v1/webhooks/stripe` running; commerce uses that `whsec_`
- [ ] `POST /v1/checkout/sessions` `{"plan_sku":"all-access-3"}` → `{ url, plan_sku, link_token }`
- [ ] Pay with test card; Stripe CLI shows `checkout.session.completed` / `customer.subscription.*` delivered (2xx)
- [ ] Pending entitlement has `stripe_subscription_id` after webhook (bot or commerce reload path)
- [ ] `all-access-10` session create also returns a hosted Checkout URL
- [ ] Unknown `plan_sku` → 400

## L2 — Local bot binding + gating

Prereq: same compose stack; bot shares `/data/entitlements.enc` with commerce. Set `ENTITLEMENTS__ENFORCE=true` **only for this L2 run** (or a throwaway env), then return to false afterward if needed.

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

- [ ] `phala ps --cvm-id 0e82fa77-8b15-4dbd-89c4-9045ab911353` shows `signal-commerce` healthy
- [ ] `GET …/health` → 200
- [ ] Dashboard webhook endpoint configured; signing secret matches `STRIPE_WEBHOOK_SECRET` on CVM
- [ ] `POST …/v1/checkout/sessions` with `Origin: https://breadchaincoop.github.io` → 200 + Checkout URL (CORS)
- [ ] Pages: Plans **Subscribe** → Stripe Checkout (requires `PUBLIC_COMMERCE_API_BASE_URL`)
- [ ] Pay test card → success page shows `!link` code → DM bot → `!enable-sigstack` in a test group
- [ ] Confirm Signal phone still registered (volume intact); prefs/entitlements still load (no “starting fresh”)
- [ ] **Rehearsal:** set `ENTITLEMENTS_ENFORCE=true` in `.phala.env`, in-place redeploy, re-check linked path + unlinked denial
- [ ] Leave enforce **true** only after rehearsal passes; otherwise set `false` and redeploy

## Pass criteria for prod enforce

All of: L1 green, L2 green (or equivalent Signal smoke against CVM), L3 smoke + enforce rehearsal green, Dashboard webhook delivering, Pages CTA live.

Then keep `ENTITLEMENTS_ENFORCE=true` on the CVM.
