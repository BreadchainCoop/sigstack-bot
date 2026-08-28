# CipherSlate site

Educational / marketing site for CipherSlate (issue [#23](https://github.com/BreadchainCoop/sigstack-bot/issues/23)).

**Stack:** SvelteKit (static) + Paraglide (en/es/fr via cookie) + Vitest + Playwright  
**Host:** GitHub Pages at `https://breadchaincoop.github.io/sigstack-bot/`  
**Design:** Small token system inspired by [Octant](https://octant.build/) accent language and [Golem Foundation](https://golem.foundation/)-like typography (DM Sans). Not affiliated with either. Products live on one `/products` page; the nav Products dropdown pins section anchors (`#language-threads`, `#in-chat`, `#transcription`).

## Local

```bash
cd site
npm install
npm run dev
```

Production base path defaults to `/sigstack-bot`. For root-local preview:

```bash
BASE_PATH= npm run build && BASE_PATH= npm run preview
```

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
3. Site URL: https://breadchaincoop.github.io/sigstack-bot/

Legacy `web/` (Private AI) is unrelated — do not use it as the product storefront.
