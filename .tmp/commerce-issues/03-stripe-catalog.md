Parent: #13

## Summary

Create Stripe Products/Prices matching the Plans page SKUs. Unblocks Checkout Session creation.

## SKUs (from site copy)

| SKU | Scope | Illustrative price |
|-----|-------|-------------------|
| `bundle-individual` | user | $5/mo |
| `bundle-group` | group | $17/mo |
| `threads-me`, `threads-group` | user / group | $2 / $11 |
| `in-chat-me`, `in-chat-all` | user / group | $2 / $11 |
| `transcription-me`, `transcription-group` | user / group | $2 / $11 |

**Decision:** confirm final prices before go-live (site currently says illustrative).

## Deliverables

- [ ] Stripe Dashboard (or API script) products created in **test** mode
- [ ] Price IDs documented in repo (e.g. `docs/commerce/stripe-catalog.md` or env example) — not secrets
- [ ] Bundle modeled as single Price per tier (simplest) vs multi-item subscription — **decision required**; recommend single Price per SKU for MVP
- [ ] Metadata convention: `plan_sku`, `scope` (`individual` | `group`)

## Depends on

Nothing (can start in parallel with entitlement store issue)
