/**
 * Must stay in sync with `crates/signal-bot/src/commands/translate_lang.rs`
 * (`ALL_LANGUAGES` + `IN_CHAT_AUTO_EXCLUDED`).
 */
export type LanguageFeature = 'threads' | 'inChatAuto' | 'manual';

export type LanguageSupport = {
	code: string;
	name: string;
	flag: string;
	languageThreads: boolean;
	inChatAuto: boolean;
	manualTranslate: boolean;
};

export const SUPPORTED_LANGUAGES: LanguageSupport[] = [
	{ code: 'en', name: 'English', flag: '🇺🇸', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'es', name: 'Spanish', flag: '🇪🇸', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'fr', name: 'French', flag: '🇫🇷', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'de', name: 'German', flag: '🇩🇪', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'it', name: 'Italian', flag: '🇮🇹', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'pt', name: 'Portuguese', flag: '🇵🇹', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'ru', name: 'Russian', flag: '🇷🇺', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'zh', name: 'Chinese', flag: '🇨🇳', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'ja', name: 'Japanese', flag: '🇯🇵', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'ko', name: 'Korean', flag: '🇰🇷', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'ar', name: 'Arabic', flag: '🇸🇦', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'hi', name: 'Hindi', flag: '🇮🇳', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'bn', name: 'Bengali', flag: '🇧🇩', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'nl', name: 'Dutch', flag: '🇳🇱', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'pl', name: 'Polish', flag: '🇵🇱', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'tr', name: 'Turkish', flag: '🇹🇷', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'vi', name: 'Vietnamese', flag: '🇻🇳', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'th', name: 'Thai', flag: '🇹🇭', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'id', name: 'Indonesian', flag: '🇮🇩', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'uk', name: 'Ukrainian', flag: '🇺🇦', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'sv', name: 'Swedish', flag: '🇸🇪', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'cs', name: 'Czech', flag: '🇨🇿', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'el', name: 'Greek', flag: '🇬🇷', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'he', name: 'Hebrew', flag: '🇮🇱', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'ro', name: 'Romanian', flag: '🇷🇴', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'hu', name: 'Hungarian', flag: '🇭🇺', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'fi', name: 'Finnish', flag: '🇫🇮', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'da', name: 'Danish', flag: '🇩🇰', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'no', name: 'Norwegian', flag: '🇳🇴', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'fa', name: 'Persian', flag: '🇮🇷', languageThreads: true, inChatAuto: true, manualTranslate: true },
	{ code: 'eu', name: 'Basque', flag: '🇪🇸', languageThreads: true, inChatAuto: false, manualTranslate: true },
	{ code: 'sw', name: 'Swahili', flag: '🇰🇪', languageThreads: true, inChatAuto: false, manualTranslate: true }
];

const FEATURE_FILTERS: Record<LanguageFeature, (lang: LanguageSupport) => boolean> = {
	threads: (lang) => lang.languageThreads,
	inChatAuto: (lang) => lang.inChatAuto,
	manual: (lang) => lang.manualTranslate
};

export const LANGUAGE_FEATURE_LABELS: Record<
	LanguageFeature,
	{ label: string; hint: string; botCommand: string }
> = {
	threads: {
		label: 'Language Threads',
		hint: '!translate-me-thread, Bilingual Threads, !list-langs',
		botCommand: '!list-langs'
	},
	inChatAuto: {
		label: 'In-chat auto-translate',
		hint: '!translate-all-on, !translate-me-on, !list-langs-in-chat',
		botCommand: '!list-langs-in-chat'
	},
	manual: {
		label: 'Manual translate (quote-reply)',
		hint: 'Reply with !translate <code> — same catalog as Language Threads',
		botCommand: '!list-langs'
	}
};

export function languagesForFeature(feature: LanguageFeature): LanguageSupport[] {
	return SUPPORTED_LANGUAGES.filter(FEATURE_FILTERS[feature]).sort((a, b) =>
		a.code.localeCompare(b.code)
	);
}

export function languageCountForFeature(feature: LanguageFeature): number {
	return languagesForFeature(feature).length;
}

export function formatLanguageLine(lang: LanguageSupport): string {
	return `${lang.flag} ${lang.code} — ${lang.name}`;
}
