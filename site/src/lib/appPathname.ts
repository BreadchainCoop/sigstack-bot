import { deLocalizeUrl } from '$lib/paraglide/runtime';

/** Deploy base — keep in sync with `paths.base` in vite.config.ts */
const DEPLOY_BASE = '/sigstack-bot';

/**
 * App pathname without kit `paths.base` or locale prefix (always starts with `/`).
 * Uses a fixed deploy base string because `$app/paths` `base` becomes relative during prerender.
 */
export function appPathname(url: URL): string {
	let path = url.pathname;
	if (path === DEPLOY_BASE || path.startsWith(`${DEPLOY_BASE}/`)) {
		path = path.slice(DEPLOY_BASE.length) || '/';
	}
	if (!path.startsWith('/')) path = `/${path}`;
	return deLocalizeUrl(new URL(path, 'https://app.local')).pathname;
}
