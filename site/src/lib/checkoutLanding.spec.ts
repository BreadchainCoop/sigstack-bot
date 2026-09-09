import { describe, expect, it } from 'vitest';
import { getContent } from '$lib/content';
import {
	LINK_CODE_MAX_LENGTH,
	linkCommand,
	planLabelFromSku,
	readLinkCode
} from './checkoutLanding';

describe('readLinkCode', () => {
	it('returns trimmed code from query', () => {
		expect(readLinkCode(new URL('https://example.com/checkout/success/?code=%20abc-1%20'))).toBe(
			'abc-1'
		);
	});

	it('returns null when missing or blank', () => {
		expect(readLinkCode(new URL('https://example.com/checkout/success/'))).toBeNull();
		expect(readLinkCode(new URL('https://example.com/checkout/success/?code='))).toBeNull();
		expect(readLinkCode(new URL('https://example.com/checkout/success/?code=%20%20'))).toBeNull();
	});

	it('returns null when over soft max length', () => {
		const tooLong = 'x'.repeat(LINK_CODE_MAX_LENGTH + 1);
		expect(
			readLinkCode(new URL(`https://example.com/checkout/success/?code=${tooLong}`))
		).toBeNull();
	});

	it('accepts a code at the soft max length', () => {
		const ok = 'y'.repeat(LINK_CODE_MAX_LENGTH);
		expect(readLinkCode(new URL(`https://example.com/checkout/success/?code=${ok}`))).toBe(ok);
	});
});

describe('planLabelFromSku', () => {
	const content = getContent('en');

	it('resolves bundle and a-la-carte offer ids', () => {
		expect(planLabelFromSku('bundle-group', content)).toBe('Bundle · Group');
		expect(planLabelFromSku('bundle-individual', content)).toBe('Bundle · Individual');
		expect(planLabelFromSku('in-chat-all', content)).toBe('In-chat · all');
		expect(planLabelFromSku('transcription-individual', content)).toBe('Transcription');
	});

	it('returns null for missing or unknown sku', () => {
		expect(planLabelFromSku(null, content)).toBeNull();
		expect(planLabelFromSku('  ', content)).toBeNull();
		expect(planLabelFromSku('not-a-real-sku', content)).toBeNull();
	});
});

describe('linkCommand', () => {
	it('prefixes !link', () => {
		expect(linkCommand('abc')).toBe('!link abc');
	});
});
