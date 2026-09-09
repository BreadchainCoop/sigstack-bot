# GitHub work index

**Only** place in this repo that may cite GitHub issue or PR numbers.

Product docs, READMEs, code comments, and plans describe work by **name** and link here (or to durable docs) when a tracker ID is needed. See [`.cursor/rules/no-issue-pr-references.mdc`](../../.cursor/rules/no-issue-pr-references.mdc).

Repo: [BreadchainCoop/sigstack-bot](https://github.com/BreadchainCoop/sigstack-bot)  
Issues: https://github.com/BreadchainCoop/sigstack-bot/issues  
Pull requests: https://github.com/BreadchainCoop/sigstack-bot/pulls

When adding work, update **this file**—do not sprinkle `#N` into other paths.

## How to use

| Need | Do |
|------|-----|
| Relate code/docs to a ticket | Link or name the topic; agents look up the number here |
| Open / comment / close a ticket | Use `gh` against the URLs below |
| Ship a PR | Tracker refs go in the **PR body** on GitHub, not in commit subjects required for the tree |

## Commerce / Stripe / entitlements

**Current epic:** [Epic: CipherSlate commerce — Stripe, entitlements, alpha](https://github.com/BreadchainCoop/sigstack-bot/issues/68) (children #53, #55–#65)  
**Superseded:** [#13](https://github.com/BreadchainCoop/sigstack-bot/issues/13) (closed — education / early site scope done)

### Related (not epic children)

| # | Title | Notes |
|---|--------|--------|
| [52](https://github.com/BreadchainCoop/sigstack-bot/issues/52) | decision: stripe checkout and webhook hosting architecture | Decision |
| [54](https://github.com/BreadchainCoop/sigstack-bot/issues/54) | ops: stripe products and prices for cipherslate plans | Catalog; SKUs follow `site/src/lib/content/en.ts` |

### Epic children (#68)

| # | Title | Notes |
|---|--------|--------|
| [53](https://github.com/BreadchainCoop/sigstack-bot/issues/53) | feat: entitlement model and encrypted store on cvm | Shipped; `entitlements.enc` / composition architecture |
| [55](https://github.com/BreadchainCoop/sigstack-bot/issues/55) | feat: stripe checkout session api from plan sku | Checkout session + success redirect query shape |
| [56](https://github.com/BreadchainCoop/sigstack-bot/issues/56) | feat: stripe webhooks and subscription lifecycle sync | |
| [57](https://github.com/BreadchainCoop/sigstack-bot/issues/57) | feat: !link and !claim-group for subscription binding | |
| [58](https://github.com/BreadchainCoop/sigstack-bot/issues/58) | feat: gate product commands on entitlements | |
| [59](https://github.com/BreadchainCoop/sigstack-bot/issues/59) | feat: alpha promo codes — 3-month bundle-all free path | `bundle-all-alpha` |
| [60](https://github.com/BreadchainCoop/sigstack-bot/issues/60) | ops: alpha code generation and revocation tooling | |
| [61](https://github.com/BreadchainCoop/sigstack-bot/issues/61) | feat: wire plans ctas to stripe checkout | |
| [62](https://github.com/BreadchainCoop/sigstack-bot/issues/62) | feat: checkout success cancel and alpha claim pages | `site/` commerce landings |
| [63](https://github.com/BreadchainCoop/sigstack-bot/issues/63) | feat: revise get-started flow and entitlement user comms | |
| [64](https://github.com/BreadchainCoop/sigstack-bot/issues/64) | feat: deploy commerce service and entitlements on phala cvm | |
| [65](https://github.com/BreadchainCoop/sigstack-bot/issues/65) | test: commerce and entitlement e2e test plan | |

**Recommended order (epic [#68](https://github.com/BreadchainCoop/sigstack-bot/issues/68)):** foundation #53+#62 (done) → parallel #63/#65 + related #52/#54 → #54→#55→#56 → #57 → #58 → #59+#60 → #61 → #64 → finish #65 before prod enforce. Full write-up on the epic issue.

## Product suite / architecture

| # | Title | Notes |
|---|--------|--------|
| [10](https://github.com/BreadchainCoop/sigstack-bot/issues/10) | Umbrella: product suite, website + Stripe, and Phala CVM split | Closed umbrella; one-CVM is current |
| [8](https://github.com/BreadchainCoop/sigstack-bot/issues/8) | Product: extract voice transcription to its own CVM (4 GB) | Closed; STT is remote Whisper, not a second CVM |
| [9](https://github.com/BreadchainCoop/sigstack-bot/issues/9) | Product: split translation into in-group vs Language Threads | Closed |
| [14](https://github.com/BreadchainCoop/sigstack-bot/issues/14) | Product: In-chat (group) translation | Closed |
| [15](https://github.com/BreadchainCoop/sigstack-bot/issues/15) | Product: Parallel Translation | Closed / retired into Language Threads |
| [16](https://github.com/BreadchainCoop/sigstack-bot/issues/16) | Product: Language Threads | Closed |
| [36](https://github.com/BreadchainCoop/sigstack-bot/issues/36) | feat: bilingual threads | Closed |

## Site / menus / i18n

| # | Title | Notes |
|---|--------|--------|
| [23](https://github.com/BreadchainCoop/sigstack-bot/issues/23) | Website: educate users with docs and product diagrams | Closed; `site/` |
| [27](https://github.com/BreadchainCoop/sigstack-bot/issues/27) | feat: localize bot menus for all !list-langs languages | Open |
| [51](https://github.com/BreadchainCoop/sigstack-bot/issues/51) | docs(site): per-product language support dropdown and eu/sw disclosure | Closed |

## Other open

| # | Title |
|---|--------|
| [40](https://github.com/BreadchainCoop/sigstack-bot/issues/40) | Add gated entry flow for new group chat members |
| [41](https://github.com/BreadchainCoop/sigstack-bot/issues/41) | Commands |
| [44](https://github.com/BreadchainCoop/sigstack-bot/issues/44) | User verification with kick for open groups |

## Pull requests

Do not list ephemeral PR numbers in product docs. For the current branch’s PR, use `gh pr view` / `gh pr list`. Add a short row here only when a PR is a durable cross-reference agents need often (rare).

| # | Title | Notes |
|---|--------|--------|
| — | — | (none pinned) |
