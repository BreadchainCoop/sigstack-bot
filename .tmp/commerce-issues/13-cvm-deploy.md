Parent: #13

## Summary

Ship entitlement enforcement to prod CVM **in place** (`phala deploy --cvm-id` — do not wipe volumes).

## Scope

- [ ] Add commerce/webhook service to [`docker/phala.yaml`](docker/phala.yaml) per hosting architecture decision
- [ ] New volume for `entitlements.enc` (or shared data mount) — document in [`docs/one-cvm-architecture.md`](docs/one-cvm-architecture.md)
- [ ] Env: `STRIPE_*`, webhook secret, `ENTITLEMENTS__ENFORCE`
- [ ] Ingress/TLS for Stripe webhooks
- [ ] Rollout: enforce=false → shadow mode → enforce=true

## Constraints

- **Do not** destroy `signal-config-translation` or `group-prefs-translation` volumes
- In-place upgrade only per AGENTS.md

## Depends on

Hosting decision, entitlement store, webhooks, command gating issues
