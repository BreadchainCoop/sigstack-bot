import { describe, expect, it } from 'vitest';
import { getContent } from './index';

describe('getContent', () => {
	it('returns English content for en', () => {
		const c = getContent('en');
		expect(c.meta.siteName).toBe('Bread Bot');
		expect(c.landing.paths.some((p) => p.primary)).toBe(true);
	});

	it('falls back to English for es and fr stubs', () => {
		expect(getContent('es').languageThreads.title).toBe(getContent('en').languageThreads.title);
		expect(getContent('fr').faq.items.length).toBeGreaterThan(0);
	});
});
