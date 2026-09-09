Parent: #13

## Summary

Replace placeholder `ctaHref: '/get-started'` on paid plans with live checkout flow.

## Scope

- [ ] Plans page bundle + à la carte paid CTAs call Checkout API
- [ ] Loading/error states
- [ ] Remove or update footnote: *"Checkout is not live yet"* in [`en.ts`](site/src/lib/content/en.ts)
- [ ] Env: checkout API base URL for static site build or runtime fetch

## Decision: static site calling API

GitHub Pages cannot hide secrets; checkout creation must call **backend** (hosting decision), not embed Stripe secret key.

## Depends on

Checkout API, hosting architecture decision
