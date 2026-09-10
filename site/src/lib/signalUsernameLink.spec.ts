import { describe, expect, it } from 'vitest';
import { resolveSignalUsernameLink } from './signalUsernameLink';

describe('resolveSignalUsernameLink', () => {
	it('returns trimmed env when set', () => {
		expect(resolveSignalUsernameLink('  https://signal.me/#eu/custom  ')).toBe(
			'https://signal.me/#eu/custom'
		);
	});

	it('returns empty when env missing or blank', () => {
		expect(resolveSignalUsernameLink(undefined)).toBe('');
		expect(resolveSignalUsernameLink(null)).toBe('');
		expect(resolveSignalUsernameLink('')).toBe('');
		expect(resolveSignalUsernameLink('   ')).toBe('');
	});
});
