import { describe, expect, it } from 'vitest';
import { resolveTheme } from './theme';

describe('resolveTheme', () => {
	it('uses stored preference when present', () => {
		expect(resolveTheme('dark')).toBe('dark');
		expect(resolveTheme('light')).toBe('light');
	});

	it('defaults to dark when nothing stored', () => {
		expect(resolveTheme(null)).toBe('dark');
	});
});
