# Stripe Subscription & `!activate` Gate

**Date:** 2026-06-15  
**Status:** Draft  
**Authors:** —  
**Related:** `x402-payment-integration.md` (superseded for consumer SaaS), `CLAUDE.md`, `2026-6-22-whisper-integration.md`, [SignalWhisperBot plans](https://signalwhisperbot.com/en/plans) (competitor reference)

---

## Overview

Bread Coop AI will operate as **traditional SaaS**: one shared Signal bot number owned by Bread Co-op, Stripe subscriptions (credit card), and personal phone numbers as customer identity. Before any feature works in a chat (DM or group), a paying subscriber must run **`!activate`** in that chat. The bot gates all commands until activation succeeds; unsubscribed or expired users receive a link to the Stripe checkout page. **DMs and groups use identical UX, commands, and gating** — no special-case group commands for activation.

This plan replaces **x402 / crypto prepaid credits** as the consumer billing path. The existing `x402-payments` crate may be reused for metering patterns but is not the v1 funding mechanism.

## Goals

1. **Single bot number** — all customers message the same Bread Co-op Signal bot
2. **Personal phone = account** — Stripe signup and usage tracking keyed to the human’s Signal number (`message.source`)
3. **`!activate` gate** — no bot features (transcribe, translate, AI, etc.) until activation succeeds **in that chat**
4. **Chat sponsor** — whoever runs `!activate` in a chat becomes the named sponsor; their subscription covers usage for everyone in that chat (free riders allowed)
5. **Unified DM/group behavior** — same commands, same activation flow, same welcome copy shape
6. **Subscription lifecycle** — friendly renewal prompt on expiry; only `!activate` (and subscribe link) until renewed
7. **Privacy preserved** — billable metadata only (phone, plan, meters); no message content persistence

### Non-Goals (v1)

- Per-customer bot phone numbers (no SIM farm / white-label bot numbers)
- Crypto / x402 consumer checkout
- Requiring every group member to subscribe
- Speaker diarization, email routing, export (competitor features — future)
- Building Stripe checkout or marketing site in this crate (separate web project)
- Metering enforcement details (minutes vs tokens) — separate follow-on plan once plans/pricing are locked

## Decisions Made

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Bot topology | **One Bread Co-op bot number** | Matches competitor UX; avoids number ops |
| Customer identity | **Subscriber personal phone (E.164)** | Matches Signal `message.source`; user confirms at Stripe |
| Billing model | **Stripe subscription** (not x402) | User-requested traditional SaaS |
| Activation | **`!activate` per chat** | Same in DM and group; establishes sponsor for that chat |
| Pre-activation behavior | **All commands blocked** except `!activate` | User requirement; returns subscribe link |
| Post-activation | **Help menu + welcome** | Confirms sponsor, subscription status |
| Group economics | **Sponsor pays, chat members ride free** | User requirement; avoids per-member subscribe UX |
| DM vs group | **Identical** | User requirement — no divergent flows |
| Entitlement source of truth | **Outside bot** (Stripe webhooks → DB/API) | Stripe cannot run inside TEE; bot reads registry |
| Message content storage | **None** (existing ephemeral model) | Privacy guarantee unchanged |
| x402 payments | **Disabled for consumer SaaS** | Keep code optional; do not expose `!deposit` in help |

> **Status:** Draft — pricing tiers and Stripe product IDs to be finalized in a follow-on session.

## Current Architecture (Baseline)

| Component | Relevance today |
|-----------|-----------------|
| `crates/signal-bot/` | Command handlers, `message.source`, `reply_target()` for DM vs group |
| `crates/x402-payments/` | Per-**sender** credit store + `UsageRecord`; AI chat deduction only; crypto deposits |
| `crates/signal-registration-proxy/` | Registers **bot** phone numbers on deployment — not end-user subscribers |
| `web/` | TEE verification + bot discovery — not billing |
| `group_preferences_store.rs` | Encrypted per-**group** prefs (transcribe/translate-all) — orthogonal to billing |

**Gap:** No subscriber registry, no Stripe integration, no per-**chat** activation state, no command gate before handlers run.

## Proposed Architecture

### High-Level Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Outside TEE (Vercel / VPS)                       │
│  Marketing site ──► Stripe Checkout (phone + plan)                       │
│       ▲              │                                                   │
│       │              ▼ webhooks                                          │
│       │         Entitlement service ──► Postgres/SQLite                  │
│       │              │  subscribers(phone, stripe_id, plan, period_end)  │
│       │              │  (optional) usage_rollup for ops dashboards         │
└───────┼──────────────┼──────────────────────────────────────────────────┘
        │              │ HTTPS read (API key / mTLS)
        ▼              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    Intel TDX CVM (docker / Phala)                          │
│  signal-api ◄──► signal-bot                                              │
│                    │                                                     │
│                    ├─► Activation gate (before handler dispatch)         │
│                    │     • chat_id = reply_target()                        │
│                    │     • sponsor = who ran !activate in this chat        │
│                    │                                                     │
│                    ├─► Entitlement cache (optional, encrypted file)      │
│                    │     periodic sync from entitlement service            │
│                    │                                                     │
│                    └─► Existing handlers (voice, translate, ask, …)     │
│                          meter usage ──► sponsor’s account                 │
└─────────────────────────────────────────────────────────────────────────┘
```

### Chat Identity (DM and Group — Same Mechanism)

Both chat types use `BotMessage::reply_target()`:

| Chat type | `reply_target()` | Activation scope |
|-----------|------------------|------------------|
| DM | Sender’s phone | One activation per subscriber DM thread |
| Group | Signal `group_id` | One activation per group |

**Sponsor:** The `message.source` of the user who successfully ran `!activate` in that `reply_target()`.

### Activation State Machine

```
                    ┌─────────────────┐
                    │  Chat inactive   │
                    │ (never activated)│
                    └────────┬────────┘
                             │ !activate
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
        Not in          Subscribed      Subscribed
        registry        + active        + expired
              │              │              │
              ▼              ▼              ▼
        Stripe link    Welcome +       Renewal link
        + help hint    !help menu      + only !activate
              │              │              │
              └──────────────┴──────────────┘
                             │
                   Other commands blocked
                   until active subscription
                   + successful !activate
```

### New / Modified Components

| Component | Type | Responsibility |
|-----------|------|----------------|
| `entitlement-service` (or `web/api`) | New service | Stripe webhooks, subscriber CRUD, HTTP API for bot |
| `crates/subscription/` or extend `signal-bot` | New / modify | Activation store, gate middleware, `!activate` handler |
| `web/` | Modify | Pricing page, Stripe Checkout links, success/cancel pages |
| `signal-bot` `main.rs` | Modify | Gate all handlers except `!activate` when chat inactive or sponsor expired |
| `x402-payments` | No change v1 | Leave disabled; optional future internal cost accounting |

### Data Models (Conceptual)

**Subscriber** (entitlement DB — not message content):

```rust
struct Subscriber {
    phone: String,              // E.164, primary key
    stripe_customer_id: String,
    stripe_subscription_id: String,
    plan_id: String,
    status: SubscriptionStatus, // active | past_due | canceled | expired
    current_period_end: DateTime<Utc>,
}
```

**Chat activation** (bot — persisted like group preferences):

```rust
struct ChatActivation {
    chat_id: String,            // reply_target(): phone or group_id
    sponsor_phone: String,      // message.source of activator
    activated_at: DateTime<Utc>,
    // Denormalized for offline/fast checks:
    sponsor_period_end: DateTime<Utc>,
}
```

**Usage event** (metering — no content):

```rust
struct UsageEvent {
    sponsor_phone: String,
    chat_id: String,
    actor_phone: String,        // who triggered (may != sponsor)
    event_type: String,         // voice_transcribe | translate | ask | …
    units: u64,                 // seconds, tokens, or weighted credits
    timestamp: DateTime<Utc>,
}
```

## Security & TEE Considerations

| Data | Where stored | Why |
|------|--------------|-----|
| Phone, plan, Stripe IDs | Entitlement DB outside TEE | Stripe integration; operational billing |
| Chat activation (sponsor per chat) | TEE encrypted file (like `group_prefs.enc`) | Survives restart; not message content |
| Usage meters | Entitlement DB and/or TEE encrypted rollup | Reconcile cost vs subscription |
| Message text / audio | Ephemeral in TEE only | Unchanged privacy model |

- **Operator can observe:** subscriber phone numbers, activation events, usage timing/sizes, which chats are active (metadata — same class as existing Signal/NEAR AI leakage).
- **Attestation:** Adding entitlement HTTP client does not require new compose services if API is external; optional sidecar would change compose hash.
- **Threat model:** This feature does **not** hide that a phone number subscribed or that a group uses the bot — only that **message contents** are not stored by the operator.

## Integration Points

| Location | Change |
|----------|--------|
| `crates/signal-bot/src/main.rs` | Wrap handler dispatch with activation + subscription gate |
| `crates/signal-bot/src/commands/activate.rs` | New `!activate` handler |
| `crates/signal-bot/src/activation_store.rs` | Per-chat activation + encrypted persistence |
| `crates/signal-bot/src/subscriber_client.rs` | HTTP client to entitlement API |
| `crates/signal-bot/src/commands/help.rs` | Only shown after successful activation (or via activate response) |
| `web/` | Stripe Checkout, `/subscribe?phone=…` deep link |
| `docker/docker-compose.yaml` | Env for entitlement API URL; optional local mock service |

### Gate Pseudocode

```rust
// Before dispatching to any handler except ActivateHandler:
fn gate(message: &BotMessage, activation: &ActivationStore, subs: &SubscriberClient) -> GateResult {
    if message.text.trim() == "!activate" {
        return GateResult::AllowActivate;
    }

    let chat_id = message.reply_target();
    let activation = activation.get(chat_id)?;

    let Some(act) = activation else {
        return GateResult::Block(subscribe_message(message.source));
    };

    let sponsor = subs.lookup(act.sponsor_phone).await?;
    if !sponsor.is_active() {
        return GateResult::Block(renew_message(&sponsor));
    }

    GateResult::Allow { sponsor: act.sponsor_phone.clone() }
}
```

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `SUBSCRIPTION__ENABLED` | `false` | Master switch for SaaS gate |
| `SUBSCRIPTION__API_URL` | — | Entitlement service base URL |
| `SUBSCRIPTION__API_KEY` | — | Bot → entitlement auth (SecretString) |
| `SUBSCRIPTION__CHECKOUT_URL` | — | Stripe / marketing checkout base (e.g. `https://breadcoop.ai/subscribe`) |
| `SUBSCRIPTION__ACTIVATION_PATH` | `/data/chat_activation.enc` | TEE-encrypted activation store |
| `SUBSCRIPTION__CACHE_TTL` | `60s` | Subscriber status cache in bot |

Compose / `.env.example` to be updated in implementation phase.

## API / User-Facing Behavior

### Signal Commands

| Command | Pre-activation | Post-activation (active sub) | Post-activation (expired sub) |
|---------|----------------|------------------------------|--------------------------------|
| `!activate` | Check registry → pay link **or** welcome + help | Re-confirm / refresh sponsor | Renewal link + “subscription expired” |
| `!help`, `!privacy`, all others | **Blocked** → subscribe link | Normal behavior | **Blocked** → renewal via `!activate` |

### `!activate` — Success Response (DM and Group)

Single template; group variant only adds “this chat” wording if needed:

```
✅ Bread Coop AI activated in this chat.

Sponsor: +1XXXXXXXXXX (you)
Plan: {plan_name} · renews {date}

{!help menu contents}

Anyone in this chat can use the bot. Usage counts against the sponsor’s subscription.
```

### `!activate` — Not Subscribed

```
This phone number is not subscribed.

Subscribe here: {checkout_url}?phone={encoded_source}

After payment, send !activate again in this chat.
```

### `!activate` — Expired Subscription

```
Your subscription ended on {date}.

Renew to keep using Bread Coop AI in this chat:
{checkout_url}?phone={encoded_sponsor}

Then send !activate again.
```

### Non-activate Command While Gated

```
This chat is not activated.

Send !activate to start. You need an active subscription:
{checkout_url}?phone={encoded_source}
```

### HTTP Endpoints (Entitlement Service — New)

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/v1/subscribers/{phone}` | Bot checks subscription status |
| `POST` | `/v1/webhooks/stripe` | Stripe events → update subscribers |
| `GET` | `/v1/health` | Health check |

Stripe webhook events (minimum): `checkout.session.completed`, `customer.subscription.updated`, `customer.subscription.deleted`, `invoice.payment_failed`.

## Dependencies

**Rust (signal-bot):**

- Existing: `reqwest` or reuse pattern from other crates for entitlement HTTP client
- Existing: `aes-gcm`, `dstack-client` for encrypted activation store (same as `group_preferences_store`)

**External:**

- Stripe (Checkout + Customer Portal + webhooks)
- Hosted entitlement API + database (can start on Vercel serverless + Supabase/PlanetScale)

**Not required v1:**

- x402 / on-chain verification

## Resource Requirements (Phala CVM)

| Resource | Impact |
|----------|--------|
| vCPU / Memory | Negligible — activation store is tiny |
| Disk | +1 small encrypted file on existing `/data` volume |
| Network | Outbound HTTPS to entitlement API + Stripe (via webhook receiver on web, not bot) |

## Files to Modify (Implementation — Future)

| File | Changes |
|------|---------|
| `crates/signal-bot/src/commands/activate.rs` | **New** — `!activate` handler |
| `crates/signal-bot/src/activation_store.rs` | **New** — per-chat sponsor persistence |
| `crates/signal-bot/src/subscriber_client.rs` | **New** — entitlement API client |
| `crates/signal-bot/src/gate.rs` | **New** — pre-dispatch subscription gate |
| `crates/signal-bot/src/main.rs` | Gate before handler loop |
| `crates/signal-bot/src/config.rs` | `SubscriptionConfig` |
| `web/` | Stripe checkout, pricing, webhook route |
| `.env.example` | Subscription env vars |
| `docker/docker-compose.yaml` | Env + optional mock entitlement service |

## Testing Plan

- [ ] Unit: gate blocks `!help` when chat inactive; allows after activate
- [ ] Unit: sponsor lookup, expired subscription blocks non-activate commands
- [ ] Unit: activation store encrypt/decrypt round-trip
- [ ] Integration: mock entitlement API + bot `!activate` flow
- [ ] Manual: Stripe test mode checkout → webhook → `!activate` in DM
- [ ] Manual: same flow in group; second user can use `!transcribe` without paying
- [ ] Manual: expired sub → renewal message; only `!activate` works
- [ ] Regression: TEE `!verify` behavior when subscription disabled (`SUBSCRIPTION__ENABLED=false`)
- [ ] Regression: existing voice/translate features unchanged when gate disabled

## Implementation Phases

### Phase 1: Entitlement service + Stripe (web)

- [ ] Stripe products/prices (plans TBD)
- [ ] Checkout page collecting **personal phone** (E.164 validation)
- [ ] Webhook handler → subscriber DB
- [ ] `GET /v1/subscribers/{phone}` for bot
- [ ] Deploy to staging (Stripe test mode)

### Phase 2: Bot activation core

- [ ] `SubscriptionConfig` + feature flag
- [ ] `ActivationStore` (encrypted persistence)
- [ ] `ActivateHandler` — success / pay / renew messages
- [ ] `gate.rs` — block all non-activate commands when inactive or expired
- [ ] Wire gate in `main.rs` before handler dispatch

### Phase 3: Subscriber client + lifecycle

- [ ] HTTP client with cache for entitlement lookups
- [ ] On `!activate`: verify `message.source` is subscribed
- [ ] Record sponsor + `chat_id` in activation store
- [ ] Expiry: detect `period_end` passed → renewal copy on any gated command
- [ ] Optional: proactive “subscription ending soon” message (cron / daily check)

### Phase 4: Usage metering (follow-on, can parallel after Phase 2)

- [ ] Emit usage events (voice seconds, NEAR tokens) against **sponsor** phone
- [ ] Enforce plan caps (minutes, groups, AI credits)
- [ ] Ops dashboard: cost vs revenue per subscriber

### Phase 5: Production hardening

- [ ] Phone normalization (`+1…` vs local formats)
- [ ] Rate-limit `!activate` abuse
- [ ] Monitor webhook failures / stale entitlement cache
- [ ] Update `!help` / `!privacy` / marketing copy
- [ ] Disable x402 consumer commands in production compose

> Detailed step-by-step tasks: create `2026-06-15-stripe-subscription-activation-implementation.md` in a follow-on session.

## Open Questions

1. **Pricing tiers** — Mirror [SignalWhisperBot](https://signalwhisperbot.com/en/plans) minute/group caps, or single paid tier for v1?
2. **Checkout phone flow** — User enters phone only on Stripe, or bot deep-link pre-fills `?phone=` from `message.source`?
3. **Second paying user in same chat** — If chat already has sponsor A, and subscriber B runs `!activate`, reject or allow takeover?
4. **Re-activation** — Required every billing period, or once per chat until sponsor lapses?
5. **`!verify` when gated** — Blocked with subscribe link, or always free as TEE marketing hook?
6. **Chat deactivation** — Explicit `!deactivate` to stop sponsoring a group, or only expiry/cancel?
7. **Grace period** — Hard stop on `period_end` or N-day grace with warning?
8. **Entitlement DB host** — Vercel serverless + managed DB vs small VPS; bot API auth (API key vs mTLS)?
9. **Relationship to x402** — Remove crate eventually, or keep for internal operator accounting only?
10. **Group count limits** — How to count “active groups” per sponsor (distinct `group_id` with valid activation)?

## References

- [SignalWhisperBot — Plans & Pricing](https://signalwhisperbot.com/en/plans)
- `docs/plans/x402-payment-integration.md` — prior payment design (crypto)
- `docs/plans/base-plan-template.md` — plan format
- `CLAUDE.md` — TEE trust model, registration proxy, data flow
- `crates/signal-bot/src/group_preferences_store.rs` — pattern for encrypted non-content persistence
- `crates/x402-payments/src/credits/store.rs` — pattern for usage records (adapt sponsor billing)

---

## Appendix: Competitor Comparison

| | SignalWhisperBot | Bread Coop AI (this plan) |
|--|------------------|---------------------------|
| Privacy claim | Legal / EU hosting | **TEE attestation** (`!verify`) |
| Billing | Stripe-style SaaS | Stripe subscription |
| Bot numbers | One shared | One shared |
| Group model | Plan limits groups | Sponsor per chat; free riders |
| Activation | (assumed on add) | Explicit `!activate` per chat |
