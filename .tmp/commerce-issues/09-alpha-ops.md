Parent: #13

## Summary

Ops need to generate, list, and revoke alpha codes for early users.

## Decision required

| Option | Description |
|--------|-------------|
| A | CLI in repo (`cargo run -p … -- generate-alpha --count 50`) |
| B | Admin HTTP endpoint (auth required) on commerce sidecar |
| C | Manual Stripe-like spreadsheet + seed script for launch batch |

Recommend **A for MVP**, **B if non-devs issue codes**.

## Deliverables

- [ ] Generate N single-use codes
- [ ] Export codes for distribution (secure channel — not committed to git)
- [ ] Revoke unused / active alpha entitlements
- [ ] Runbook in `docs/` for alpha program

## Depends on

Alpha promo codes issue (schema/format)
