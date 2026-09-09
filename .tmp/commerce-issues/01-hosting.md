Parent: #13

## Problem

The static `site/` (GitHub Pages) cannot hold Stripe secrets or receive webhooks. We need a runtime for:
- Creating Checkout Sessions from plan SKUs
- Receiving Stripe webhooks and updating entitlements
- Optionally proxying Customer Portal links

## Decision required

Pick one primary approach (hybrid allowed if justified):

| Option | Pros | Cons |
|--------|------|------|
| **A. CVM sidecar** — new service in `docker/phala.yaml` next to `signal-bot` | Single deploy; entitlements written directly to TEE volume; simpler trust story | Stripe secret in CVM footprint; must expose HTTPS ingress for webhooks |
| **B. External serverless** (Cloudflare Worker, Fly, etc.) | Familiar Stripe patterns; no webhook port on CVM | Needs secure channel to push entitlements into TEE (API + auth) |
| **C. Hybrid** — Checkout API on serverless, webhook writes via authenticated CVM API | Split concerns | Two services to operate |

## Deliverables

- [ ] Written decision in issue comment (link to `docs/plans/` ADR if needed)
- [ ] Sequence diagram: Plans CTA → Checkout → webhook → entitlement store → bot gate
- [ ] Notes on Phala ingress / TLS for webhook URL
- [ ] Unblocks checkout API and CVM deploy child issues (see parent #13 breakdown)

## References

- [`site/README.md`](site/README.md) — static site, no API routes
- [`docs/one-cvm-architecture.md`](docs/one-cvm-architecture.md) — one CVM, volumes
- [`.agents/skills/stripe-best-practices/`](.agents/skills/stripe-best-practices/)
