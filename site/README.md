# Sigstack site

Educational / marketing site for Sigstack.

**Stack:** SvelteKit (static) + Paraglide (en/es/fr via cookie) + Vitest + Playwright  
**Host:** GitHub Pages at `https://breadchaincoop.github.io/sigstack-bot/sigstack/`  
**Design:** Small token system inspired by [Octant](https://octant.build/) accent language and [Golem Foundation](https://golem.foundation/)-like typography (DM Sans). Not affiliated with either. Products live on one `/products` page; the nav Products dropdown pins section anchors (`#language-threads`, `#in-chat`, `#transcription`).

## Local

```bash
cd site
npm install
npm run dev
```

Production base path defaults to `/sigstack-bot/sigstack`. For root-local preview:

```bash
BASE_PATH= npm run build && BASE_PATH= npm run preview
```

## Commerce landings

Static post-checkout and alpha routes (no checkout API required yet):

| Route | Purpose |
| ----- | ------- |
| `/checkout/success/` | After Stripe Checkout — show `!link` from `?code=` and plan from `?plan=` |
| `/checkout/cancel/` | Canceled checkout — return to Plans |
| `/alpha/` | Alpha explainer + inline code → Signal `!link` instructions (no post-submit navigation; `?code=` still hydrates shared links) |

Expected Stripe redirect shapes:

```text
…/checkout/success/?code=<link_token>&plan=<plan_sku>
…/checkout/cancel/
```

Optional public env (GitHub Pages-safe; never put secret keys here):

| Variable | Effect |
| -------- | ------ |
| `PUBLIC_STRIPE_PORTAL_URL` | Success page “Manage billing” link; if unset, shows “coming soon” stub |
| `PUBLIC_SIGNAL_USERNAME_TOKEN` | Alpha / checkout success “Message Sigstack” button — token only; site builds `signal.me/#eu/<token>` (never put the bot E.164 or a full URL here; `#` breaks unquoted `.env`) |

Set via `site/.env` (gitignored; copy from [`.env.example`](.env.example)) or the GitHub Actions variable for Pages. Empty env hides the Message CTA (missing-link copy). After the bot claims or re-claims its username, paste the logged `username_token` (or second line of `!bot-username`) into env / the Actions var and redeploy Pages — do not commit real tokens. TEE capture: [DEVELOPMENT.md — Signal username → site Message button](../.agents/docs/DEVELOPMENT.md#signal-username-site-message-button).

## Scripts

| Script              | Purpose                   |
| ------------------- | ------------------------- |
| `npm run check`     | `svelte-check`            |
| `npm run test:unit` | Vitest (unit + component) |
| `npm run test:e2e`  | Playwright smoke + axe    |
| `npm run build`     | Static build → `build/`   |

## Enable GitHub Pages

1. Repo **Settings → Pages → Build and deployment → Source: GitHub Actions**
2. Merge to `main` (or run **Pages** workflow manually)
3. Site URL: https://breadchaincoop.github.io/sigstack-bot/sigstack/

Legacy `web/` (Private AI) is unrelated — do not use it as the product storefront.
