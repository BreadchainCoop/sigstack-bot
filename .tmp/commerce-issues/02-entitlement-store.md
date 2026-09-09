Parent: #13

## Summary

Add a first-class **entitlement** layer separate from feature prefs (`group_prefs.enc`). The bot enforces access before paid/alpha features and NEAR inference.

## Scope

- [ ] Schema for entitlements (encrypted snapshot on CVM volume, e.g. `/data/entitlements.enc`)
- [ ] Record types:
  - **Individual scope:** keyed by Signal user UUID (phone fallback)
  - **Group scope:** keyed by group `internal_id`
- [ ] Fields: `plan_sku`, `source` (`stripe` | `alpha`), `status` (`active` | `past_due` | `expired` | `canceled`), `expires_at`, `stripe_customer_id` / `subscription_id` (nullable), `owner_uuid`, `link_token` (pending link), `claimed_group_id` (nullable)
- [ ] Plan SKU enum aligned with [`site/src/lib/content/en.ts`](site/src/lib/content/en.ts): `bundle-individual`, `bundle-group`, à la carte SKUs
- [ ] CRUD API internal to `signal-bot` (or shared crate) used by webhook service and commands
- [ ] Same encryption pattern as [`group_preferences_store.rs`](crates/signal-bot/src/group_preferences_store.rs) (dstack DeriveKey + volume persistence)

## Decision: plan composition

When user has Bundle-all alpha but later buys à la carte — document precedence (alpha grants bundle-all; paid subscription replaces on overlap).

## Out of scope

- Stripe wiring (webhooks child issue)
- Command handlers (`!link` child issue)
- Command gating (entitlement gate child issue)

## Acceptance

- [ ] Unit tests for store load/save, expiry, link binding
- [ ] Volume documented in `docs/one-cvm-architecture.md`
