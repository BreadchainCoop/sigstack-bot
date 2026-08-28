import { describe, expect, it } from 'vitest';
import { appPathname } from './appPathname';

describe('appPathname', () => {
	it('strips kit base and locale prefix', () => {
		expect(appPathname(new URL('https://x.test/sigstack-bot/'))).toBe('/');
		expect(appPathname(new URL('https://x.test/sigstack-bot/suite/')).replace(/\/$/, '')).toBe(
			'/suite'
		);
		expect(appPathname(new URL('https://x.test/sigstack-bot/es/suite/')).replace(/\/$/, '')).toBe(
			'/suite'
		);
	});
});
