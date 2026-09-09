Parent: #13

## Summary

Post-checkout and alpha onboarding routes on `site/`.

## Routes

- [ ] `/checkout/success` — show `!link <code>` instructions, plan purchased, link to Customer Portal
- [ ] `/checkout/cancel` — return to Plans, no entitlement created
- [ ] `/alpha` (or `/get-started/alpha`) — explain alpha program, optional code entry that redirects to Signal instructions

## Decision: show link code on success page

| Option | Notes |
|--------|-------|
| A | Display code from URL query (`?code=`) returned by checkout API redirect |
| B | Email code via Stripe customer email only |

Recommend **A + B** (Stripe receipt + on-page code).

## Note

Static routes and copy can ship before checkout API lands; wire `?code=` query param when checkout API is ready.

## Depends on

Checkout API and alpha codes issues (for full wiring; static UI can start now)
