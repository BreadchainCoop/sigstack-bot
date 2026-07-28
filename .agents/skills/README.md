# Vendored agent skills

Third-party [SkillsMP](https://skillsmp.com/search) / GitHub skills installed for Cursor via `npx skills add … -a cursor --copy`.

These are **domain** skills (Rust, Docker, Stripe). Feature workflow uses the Compound Engineering marketplace plugin (`/ce-brainstorm`, `/ce-plan`, `/ce-work`, …) — see [AGENTS.md](../../AGENTS.md). Prefer CE for planning/shipping; reach for these skills when writing or reviewing stack-specific code.

**Review before trust** — skills run with full agent permissions. Prefer project docs under `.agents/docs/` and `docs/` for Signal/TEE domain rules.

| Skill | Use for | Source |
|-------|---------|--------|
| [`rust-patterns`](rust-patterns/) | Idiomatic Rust, ownership, errors, async | [affaan-m/ECC](https://github.com/affaan-m/ECC/tree/main/skills/rust-patterns) · [SkillsMP](https://skillsmp.com/creators/affaan-m/ecc/skills-rust-patterns) |
| [`rust-testing`](rust-testing/) | Unit/async tests, mockall, TDD habits | [affaan-m/ECC](https://github.com/affaan-m/ECC/tree/main/skills/rust-testing) |
| [`docker`](docker/) | Dockerfiles, compose, networking, hardening | [rbaumier/skills](https://github.com/rbaumier/skills) · [SkillsMP](https://skillsmp.com/skills/rbaumier-skills-docker-skill-md) |
| [`stripe-best-practices`](stripe-best-practices/) | Checkout, subscriptions, webhooks (future site) | [midudev/autoskills](https://github.com/midudev/autoskills) · [SkillsMP](https://skillsmp.com/creators/midudev/autoskills/packages-autoskills-skills-registry-stripe-best-practices) |

Reinstall / update:

```bash
npx skills add affaan-m/ECC --skill rust-patterns --skill rust-testing -a cursor --copy -y
npx skills add rbaumier/skills --skill docker -a cursor --copy -y
npx skills add midudev/autoskills --skill stripe-best-practices -a cursor --copy -y
```

Domain ops (TEE, `BOT__ROLE`, dual CVM): [../docs/DEVELOPMENT.md](../docs/DEVELOPMENT.md). Agent entrypoint: [../../AGENTS.md](../../AGENTS.md). Cursor project rules: [../../.cursor/rules/](../../.cursor/rules/).
