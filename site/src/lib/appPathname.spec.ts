import { describe, expect, it } from 'vitest';
import { appPathname } from './appPathname';

describe('appPathname', () => {
	it('strips kit base and locale prefix', () => {
		expect(appPathname(new URL('https://x.test/sigstack-bot/'))).toBe('/');
		expect(appPathname(new URL('https://x.test/sigstack-bot/faq/')).replace(/\/$/, '')).toBe(
			'/faq'
		);
		expect(appPathname(new URL('https://x.test/sigstack-bot/es/faq/')).replace(/\/$/, '')).toBe(
			'/faq'
		);
	});
});
