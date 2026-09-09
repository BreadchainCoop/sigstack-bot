Parent: #13

## Summary

Webhook handler keeps entitlements in sync with Stripe. Source of truth for paid status is Stripe; enforcement copy lives in TEE (entitlement store issue).

## Events to handle

- [ ] `checkout.session.completed` — activate entitlement, preserve `link_token`
- [ ] `customer.subscription.updated` — plan changes, `past_due`
- [ ] `customer.subscription.deleted` — `canceled`
- [ ] `invoice.payment_failed` — grace period policy (**decision:** hard stop vs N-day grace)

## Scope

- [ ] Webhook signature verification
- [ ] Idempotency (Stripe event IDs)
- [ ] Write through to entitlement store

## Decision: grace period

| Option | Behavior |
|--------|----------|
| A | Immediate feature block on `past_due` |
| B | 3–7 day grace, then block |
| C | Block NEAR calls only, keep prefs |

Recommend **B** for MVP; document in issue closure.

## Depends on

Hosting decision, entitlement store, checkout API (see parent #13 breakdown)
