export type Theme = 'light' | 'dark';

export const STORAGE_KEY = 'theme';

export function getStoredTheme(): Theme | null {
	if (typeof localStorage === 'undefined') return null;
	const raw = localStorage.getItem(STORAGE_KEY);
	return raw === 'light' || raw === 'dark' ? raw : null;
}

/** Resolve effective theme from stored preference; default dark when unset. */
export function resolveTheme(stored: Theme | null, _prefersDark = false): Theme {
	if (stored) return stored;
	return 'dark';
}

export function applyTheme(theme: Theme): void {
	document.documentElement.dataset.theme = theme;
}

export function toggleTheme(): Theme {
	const current =
		(document.documentElement.dataset.theme as Theme | undefined) ??
		resolveTheme(getStoredTheme());
	const next: Theme = current === 'dark' ? 'light' : 'dark';
	localStorage.setItem(STORAGE_KEY, next);
	applyTheme(next);
	return next;
}

export function initThemeFromDocument(): Theme {
	const theme = resolveTheme(getStoredTheme());
	applyTheme(theme);
	return theme;
}
