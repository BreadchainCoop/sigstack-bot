import type { SiteContent } from '$lib/content/types';

/** Soft cap so junk query strings are not rendered into the page. */
export const LINK_CODE_MAX_LENGTH = 128;

/**
 * Read and sanitize `?code=` from a landing URL.
 * Returns null when missing, blank, or over the soft max length.
 */
export function readLinkCode(url: URL): string | null {
	const raw = url.searchParams.get('code');
	if (raw == null) return null;
	const code = raw.trim();
	if (!code) return null;
	if (code.length > LINK_CODE_MAX_LENGTH) return null;
	return code;
}

/**
 * Map a plan SKU / offer id from `?plan=` to a display name from site content.
 */
export function planLabelFromSku(sku: string | null, content: SiteContent): string | null {
	if (!sku) return null;
	const id = sku.trim();
	if (!id) return null;

	const fromBundle = content.pages.plans.bundle.offers.find((o) => o.id === id);
	if (fromBundle) return fromBundle.name;

	for (const product of content.pages.plans.products) {
		const offer = product.offers.find((o) => o.id === id);
		if (offer) return offer.name;
	}

	return null;
}

/** Format the Signal command shown on success / alpha landings. */
export function linkCommand(code: string): string {
	return `!link ${code}`;
}

/**
 * Copy `!link <code>` to the clipboard.
 * Returns false when Clipboard API is unavailable or write fails.
 */
export async function copyLinkCommand(code: string): Promise<boolean> {
	if (typeof navigator === 'undefined' || !navigator.clipboard?.writeText) {
		return false;
	}
	try {
		await navigator.clipboard.writeText(linkCommand(code));
		return true;
	} catch {
		return false;
	}
}
