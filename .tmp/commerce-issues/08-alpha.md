Parent: #13

## Summary

Alpha users get **full CipherSlate Bundle** (all products, individual + group scope) for **90 days** without Stripe. Must coexist with paid gating.

## Scope

- [ ] Alpha code format + storage (hashed codes in entitlement store or separate alpha registry)
- [ ] Redeem via same `!link <code>` flow with `source: alpha`
- [ ] On redeem: create entitlement `plan_sku: bundle-all-alpha`, `expires_at: now + 90 days`, scopes = full bundle
- [ ] Expiry job or lazy check on each gate
- [ ] Optional: single-use vs multi-use codes (**decision** — recommend **single-use** per code, batch-generated)

## Website path

Alpha users may claim from site or go straight to Signal — both must work (success/alpha pages issue).

## Acceptance

- [ ] Alpha user can use all products for 90 days
- [ ] After expiry, user sees subscribe CTA, not a crash/error loop
- [ ] Paid subscribers unaffected

## Depends on

Entitlement store, `!link` / `!claim-group` issues
