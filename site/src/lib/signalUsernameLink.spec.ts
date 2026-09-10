import { describe, expect, it } from 'vitest';
import {
	E2E_SIGNAL_USERNAME_LINK,
	E2E_SIGNAL_USERNAME_TOKEN,
	resolveSignalUsernameLink,
	SIGNAL_USERNAME_LINK_PREFIX
} from './signalUsernameLink';

describe('resolveSignalUsernameLink', () => {
	it('assembles href from token', () => {
		expect(resolveSignalUsernameLink('  abcTOKEN  ')).toBe(
			`${SIGNAL_USERNAME_LINK_PREFIX}abcTOKEN`
		);
	});

	it('returns empty when env missing or blank', () => {
		expect(resolveSignalUsernameLink(undefined)).toBe('');
		expect(resolveSignalUsernameLink(null)).toBe('');
		expect(resolveSignalUsernameLink('')).toBe('');
		expect(resolveSignalUsernameLink('   ')).toBe('');
	});

	it('rejects full URLs and placeholders', () => {
		expect(resolveSignalUsernameLink('https://signal.me/#eu/abc')).toBe('');
		expect(resolveSignalUsernameLink('https://signal.me/')).toBe('');
		expect(resolveSignalUsernameLink('YOUR_USERNAME_TOKEN_HERE')).toBe('');
		expect(resolveSignalUsernameLink('a/b')).toBe('');
	});

	it('e2e stubs stay consistent', () => {
		expect(E2E_SIGNAL_USERNAME_LINK).toBe(
			`${SIGNAL_USERNAME_LINK_PREFIX}${E2E_SIGNAL_USERNAME_TOKEN}`
		);
		expect(resolveSignalUsernameLink(E2E_SIGNAL_USERNAME_TOKEN)).toBe(E2E_SIGNAL_USERNAME_LINK);
	});
});
