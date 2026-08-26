import type { SiteContent } from './types';
import { en } from './en';

/** FR/ES stubs fall back to English until copy is localized. */
export function getContent(locale: string): SiteContent {
	switch (locale) {
		case 'en':
		case 'es':
		case 'fr':
			return en;
		default:
			return en;
	}
}
