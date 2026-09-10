/**
 * Assemble Message Sigstack href from PUBLIC_SIGNAL_USERNAME_TOKEN.
 * Empty when unset/invalid — LinkInSignal shows the missing-link copy.
 * Set via site/.env (local), GH Actions var (Pages), or Playwright webServer env (e2e).
 * Token only — never a full URL (dotenv would truncate at `#`).
 */
export const SIGNAL_USERNAME_LINK_PREFIX = 'https://signal.me/#eu/';

export function resolveSignalUsernameLink(fromEnv?: string | null): string {
	const token = fromEnv?.trim() ?? '';
	if (!token) return '';
	if (token.includes('#') || token.includes('/') || /^https?:/i.test(token)) return '';
	if (token === 'YOUR_USERNAME_TOKEN_HERE') return '';
	return `${SIGNAL_USERNAME_LINK_PREFIX}${token}`;
}

/** Fake share token for Playwright only — not a real bot. */
export const E2E_SIGNAL_USERNAME_TOKEN = 'e2e-test-username-token';

/** Assembled href for Playwright assertions. */
export const E2E_SIGNAL_USERNAME_LINK = `${SIGNAL_USERNAME_LINK_PREFIX}${E2E_SIGNAL_USERNAME_TOKEN}`;
