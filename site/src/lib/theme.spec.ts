import { describe, expect, it } from 'vitest';
import { resolveTheme } from './theme';

describe('resolveTheme', () => {
	it('uses stored preference when present', () => {
		expect(resolveTheme('dark', false)).toBe('dark');
		expect(resolveTheme('light', true)).toBe('light');
	});

	it('defaults to dark when nothing stored', () => {
		expect(resolveTheme(null, true)).toBe('dark');
		expect(resolveTheme(null, false)).toBe('dark');
	});
});
