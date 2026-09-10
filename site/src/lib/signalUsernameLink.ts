/**
 * Resolve Message Sigstack href from PUBLIC_SIGNAL_USERNAME_LINK.
 * Empty when unset — LinkInSignal shows the missing-link copy (never commit a real share link).
 * Set via site/.env (local), GH Actions var (Pages), or Playwright webServer env (e2e).
 */
export function resolveSignalUsernameLink(fromEnv?: string | null): string {
	return fromEnv?.trim() ?? '';
}

/** Fake share link for Playwright only — not a real bot. */
export const E2E_SIGNAL_USERNAME_LINK = 'https://signal.me/#eu/e2e-test-username-link';
