import { describe, expect, it } from 'vitest';
import {
	SUPPORTED_LANGUAGES,
	languageCountForFeature,
	languagesForFeature
} from './languages';

describe('languages catalog', () => {
	it('lists 32 Language Threads languages including Basque and Swahili', () => {
		expect(SUPPORTED_LANGUAGES).toHaveLength(32);
		const eu = SUPPORTED_LANGUAGES.find((l) => l.code === 'eu');
		const sw = SUPPORTED_LANGUAGES.find((l) => l.code === 'sw');
		expect(eu?.name).toBe('Basque');
		expect(sw?.name).toBe('Swahili');
		expect(eu?.languageThreads).toBe(true);
		expect(sw?.languageThreads).toBe(true);
	});

	it('excludes Basque and Swahili from in-chat auto', () => {
		expect(languageCountForFeature('inChatAuto')).toBe(30);
		const inChat = languagesForFeature('inChatAuto');
		expect(inChat.some((l) => l.code === 'eu')).toBe(false);
		expect(inChat.some((l) => l.code === 'sw')).toBe(false);
		expect(inChat.some((l) => l.code === 'es')).toBe(true);
	});

	it('includes all threads languages in manual translate', () => {
		expect(languageCountForFeature('manual')).toBe(32);
		expect(languageCountForFeature('threads')).toBe(32);
	});
});
