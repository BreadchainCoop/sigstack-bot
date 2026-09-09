Parent: #13

## Summary

API endpoint: `POST /checkout` (or equivalent) accepts `plan_sku`, returns Stripe Checkout URL. Creates pending entitlement with generated `link_token`.

## Scope

- [ ] Map `plan_sku` → Stripe Price ID (from Stripe catalog ops issue)
- [ ] `checkout.sessions.create` with `mode: subscription`
- [ ] Generate cryptographically random `link_token`; store pending entitlement (entitlement store issue)
- [ ] `success_url` / `cancel_url` pointing to site routes (success/cancel pages issue)
- [ ] Session metadata: `plan_sku`, `link_token`

## Decision required

Blocked until hosting architecture decision issue closes (CVM sidecar vs external).

## Acceptance

- [ ] Test mode: session opens Stripe Checkout for each SKU
- [ ] Pending entitlement exists before webhook completes (or on `checkout.session.completed` only — **decide**)
