# Stripe catalog — Sigstack all-access packs

MVP paid catalog: **two** monthly Prices on one Product. Alpha (`bundle-all-alpha`) is code-based and has **no** Stripe Product.

Checkout / webhooks map `plan_sku` → Price ID. Price IDs are not secrets; `STRIPE_SECRET_KEY` never belongs in git.

## SKUs

| plan_sku | Site offer | Amount | max_groups | Who is covered |
|----------|------------|--------|------------|----------------|
| `all-access-3` | All-access · 3 groups | $10/mo | 3 | All members in each enabled Signal group |
| `all-access-10` | All-access · 10 groups | $25/mo | 10 | Same |
| `bundle-all-alpha` | Alpha band (site) | Free | Unlimited | Same (redeem + `!link`) |

## Modeling

- **Single Price per SKU** (not multi-item Subscription line items).
- One Product: **Sigstack All-Access**; two recurring monthly Prices.
- Price metadata (set by seed script):
  - `plan_sku` — `all-access-3` \| `all-access-10`
  - `max_groups` — `3` \| `10`
  - `scope` — `group`
- Price `lookup_key`: `all-access-3-monthly` / `all-access-10-monthly` (idempotent re-seed).

## Seed (test mode)

Requires a **test** secret key (`sk_test_…`):

```bash
STRIPE_SECRET_KEY=sk_test_... node scripts/stripe-seed-catalog.mjs
# after a successful run, persist IDs into this file:
STRIPE_SECRET_KEY=sk_test_... node scripts/stripe-seed-catalog.mjs --write-docs
```

Refuse live keys in the script. Duplicate live catalog later with a live key + a separate live section if needed.

<!-- stripe-catalog:generated:start -->
## Test catalog (seeded)

Product: **Sigstack All-Access** (`prod_VLrFdkiQzlcrpp`)

| plan_sku | lookup_key | amount | max_groups | Price ID (test) |
|----------|------------|--------|------------|-----------------|
| `all-access-3` | `all-access-3-monthly` | $10/mo | 3 | `price_1UL9WmKDFZS5PEar2Y7pyhN0` |
| `all-access-10` | `all-access-10-monthly` | $25/mo | 10 | `price_1UL9WmKDFZS5PEarCbTlmIyn` |

Env placeholders for checkout (IDs are public; secret key stays out of git):

```bash
STRIPE_PRICE_ALL_ACCESS_3=price_1UL9WmKDFZS5PEar2Y7pyhN0
STRIPE_PRICE_ALL_ACCESS_10=price_1UL9WmKDFZS5PEarCbTlmIyn
```

<!-- stripe-catalog:generated:end -->

## Related code

- Bot `PlanSku`: `crates/signal-bot/src/entitlements_store.rs` (`AllAccess3`, `AllAccess10`, `BundleAllAlpha`)
- Site offers: `site/src/lib/content/en.ts` (`pages.plans.paid.offers`)
