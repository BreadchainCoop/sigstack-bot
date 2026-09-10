import { describe, expect, it } from 'vitest';
import { getContent } from './index';

describe('getContent', () => {
	it('returns English content for en', () => {
		const c = getContent('en');
		expect(c.meta.siteName).toBe('Sigstack');
		expect(c.pages.home.title).toBe('Sigstack');
		expect(c.pages.home.lead).not.toContain('Placeholder');
		expect(c.pages.home.paths).toHaveLength(3);
		expect(c.pages.home.paths[0]?.href).toBe('/products#language-threads');
		expect(c.pages.getStarted.steps).toHaveLength(3);
		expect(c.pages.getStarted.hubCommands.length).toBeGreaterThan(0);
		expect(c.pages.products.sections).toHaveLength(3);
		expect(c.legalLicense.title).toBe('Apache License 2.0');
		expect(c.legalLicense.fullText).toContain('Apache License');
	});

	it('falls back to English for es and fr stubs', () => {
		expect(getContent('es').pages.products.title).toBe(getContent('en').pages.products.title);
		expect(getContent('fr').pages.plans.title).toBe(getContent('en').pages.plans.title);
	});
});
