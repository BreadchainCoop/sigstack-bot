# Stripe Integration TODO

Scenario **A**: an existing Checkout Session create call was found and updated in place.

## Values to Replace

No `sample_only` placeholders were introduced. Existing real values were preserved:

| Field | Current Value | Status |
|-------|--------------|--------|
| mode | `subscription` | Keep — matches recurring plan SKUs (`all-access-3` / `all-access-10`). |
| success_url | Built from `SITE__PUBLIC_BASE_URL` + `/checkout/success/?code=…&plan=…` | Keep — set via site public base URL env. |
| cancel_url | Built from `SITE__PUBLIC_BASE_URL` + `/checkout/cancel/` | Keep — set via site public base URL env. |
| line_items[].price | From `STRIPE__PRICE_ALL_ACCESS_3` / `STRIPE__PRICE_ALL_ACCESS_10` | Keep — configure real Price IDs in env if not already set. |

**ui_mode note:** This project uses a raw HTTP (reqwest) Stripe client, not a versioned Stripe SDK. `ui_mode` was set to `hosted_page` (SDK ≥ 21.0.0 / current API). If you later adopt a Stripe SDK below 21.0.0, change `ui_mode` to `hosted`.

**Files containing these session parameters:**
- [crates/signal-commerce/src/stripe_client.rs](crates/signal-commerce/src/stripe_client.rs)

## Configured Parameters

These parameters were configured in Checkout Studio and are set on the existing create call.

**Files containing these parameters:**
- [crates/signal-commerce/src/stripe_client.rs](crates/signal-commerce/src/stripe_client.rs)

| Parameter | Value |
|-----------|-------|
| ui_mode | hosted_page |
| billing_address_collection | auto |
| phone_number_collection.enabled | false |
| automatic_tax.enabled | false |
| allow_promotion_codes | false |
| payment_method_collection | always |
| submit_type | auto |
| integration_identifier | hosted_web_0001 |
| origin_context | web |

Product fulfillment fields (`metadata`, `subscription_data.metadata`, `client_reference_id`) were left unchanged so link-token / webhook entitlement attachment keeps working.

## Setup and next steps

### Environment variables

Ensure commerce env matches code (see `docker/.env.example` / commerce config):

- `STRIPE__SECRET_KEY` — restricted key preferred (`rk_…`) over `sk_…`
- `STRIPE__WEBHOOK_SECRET` — from Dashboard workbench webhooks
- `STRIPE__PRICE_ALL_ACCESS_3` / `STRIPE__PRICE_ALL_ACCESS_10` — Dashboard Price IDs
- `SITE__PUBLIC_BASE_URL` — used for Checkout success/cancel URLs

Do not put secret keys in client/Vite code. Server-only vars must not use a `VITE_` prefix.

### Project structure

No new routes or crates. Only the form body of `StripeClient::create_checkout_session` in `crates/signal-commerce/src/stripe_client.rs` was updated, plus this TODO file.

### How the integration works

1. Site/API calls commerce `POST /v1/checkout/sessions` with a plan SKU.
2. Commerce creates a pending entitlement + Stripe Checkout Session (`mode: subscription`, hosted page).
3. Customer pays on Stripe-hosted Checkout.
4. Webhook `checkout.session.completed` attaches Stripe customer/subscription IDs via `metadata.link_token`.
5. User completes `!link` / Signal linking against the pending entitlement.

### Testing

- Use [test mode](https://dashboard.stripe.com/test/apikeys) keys.
- Card: `4242 4242 4242 4242`, any future expiry, any CVC.
- Decline: `4000 0000 0000 0002`.
- Forward webhooks locally with Stripe CLI if needed: `stripe listen --forward-to …`.

### Next steps

1. Confirm Price IDs and secrets are set in the commerce env for the target Stripe account.
2. Verify hosted Checkout in test mode end-to-end (create session → pay → webhook → link).
3. If you enable Stripe Tax later, turn on `automatic_tax` only after active tax registrations exist.
4. Fulfillment and entitlement logic already live in `signal-commerce` / `entitlements_store` — extend there, not in a new sample server.

### Resources

- https://support.stripe.com
- https://docs.stripe.com/mcp
- https://docs.stripe.com/payments/checkout
- https://docs.stripe.com/keys-best-practices
