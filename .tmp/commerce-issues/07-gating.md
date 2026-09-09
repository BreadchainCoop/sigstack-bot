Parent: #13

## Summary

Before enabling paid features or calling NEAR AI for paid products, check entitlement store. **Keep `AcceptAll` invite policy** — gate features, not joins.

## Gating map

| Command / feature | Required entitlement |
|-------------------|---------------------|
| `!translate-me-on`, `!translate-me-thread` | individual SKU or bundle-individual or alpha bundle-all |
| `!translate-all-on` | group SKU or bundle-group or alpha (group features) |
| Group Language Threads setup | `threads-group` or bundle-group or alpha |
| `!transcribe-on` (auto) | transcription SKU or bundle or alpha |

Use existing identity keys from [`group_preferences_store.rs`](crates/signal-bot/src/group_preferences_store.rs).

## Alpha + paid coexistence

- [ ] Active alpha (unexpired) grants bundle-all — paid path must not break alpha users
- [ ] Expired alpha → fall through to paid check or deny with subscribe CTA

## Acceptance

- [ ] Tests per command path: entitled, unlinked, expired, wrong scope
- [ ] No regression for ops/dev when entitlements disabled via env flag (**decision:** `ENTITLEMENTS__ENFORCE=false` for local dev)

## Depends on

Entitlement store, `!link` / `!claim-group`, alpha codes issues
