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
| `/alpha/` | Alpha program explainer + optional code entry |

Expected Stripe redirect shapes:

```text
…/checkout/success/?code=<link_token>&plan=<plan_sku>
…/checkout/cancel/
```

Optional public env (GitHub Pages-safe; never put secret keys here):

| Variable | Effect |
| -------- | ------ |
| `PUBLIC_STRIPE_PORTAL_URL` | Success page “Manage billing” link; if unset, shows “coming soon” stub |
| `PUBLIC_SIGNAL_USERNAME_LINK` | Alpha / checkout success “Message Sigstack” button (`signal.me/#eu/…` username share link; never put the bot E.164 here) |

After the bot claims its Signal username (startup `BOT__SIGNAL_USERNAME=sigstack`), copy the logged `username_link` into the GitHub Actions variable `PUBLIC_SIGNAL_USERNAME_LINK` and redeploy Pages.

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
