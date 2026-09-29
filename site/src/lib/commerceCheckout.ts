/** Browser helper: create a Stripe Checkout Session via signal-commerce. */

export type CreateCheckoutSuccess = {
	ok: true;
	url: string;
	plan_sku: string;
	link_token: string;
};

export type CreateCheckoutFailure = {
	ok: false;
	error: string;
};

export type CreateCheckoutResult = CreateCheckoutSuccess | CreateCheckoutFailure;

/** Trim and strip a trailing slash; null when unset/blank. */
export function commerceApiBase(envValue: string | undefined | null): string | null {
	const trimmed = envValue?.trim();
	if (!trimmed) return null;
	return trimmed.replace(/\/+$/, '');
}

type CheckoutResponseBody = {
	url?: string;
	plan_sku?: string;
	link_token?: string;
	error?: string;
};

/**
 * POST `{ plan_sku }` to `${base}/v1/checkout/sessions`.
 * On success returns the Stripe hosted Checkout URL for redirect.
 */
export async function createCheckoutSession(
	baseUrl: string,
	planSku: string,
	fetchImpl: typeof fetch = fetch
): Promise<CreateCheckoutResult> {
	const base = commerceApiBase(baseUrl);
	if (!base) {
		return { ok: false, error: 'checkout API base URL is not configured' };
	}

	let response: Response;
	try {
		response = await fetchImpl(`${base}/v1/checkout/sessions`, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				Accept: 'application/json'
			},
			body: JSON.stringify({ plan_sku: planSku })
		});
	} catch {
		return { ok: false, error: 'network error starting checkout' };
	}

	let body: CheckoutResponseBody = {};
	try {
		body = (await response.json()) as CheckoutResponseBody;
	} catch {
		// non-JSON body
	}

	if (!response.ok) {
		const msg =
			typeof body.error === 'string' && body.error.trim()
				? body.error.trim()
				: `checkout failed (${response.status})`;
		return { ok: false, error: msg };
	}

	const url = typeof body.url === 'string' ? body.url.trim() : '';
	if (!url) {
		return { ok: false, error: 'checkout response missing url' };
	}

	return {
		ok: true,
		url,
		plan_sku: typeof body.plan_sku === 'string' ? body.plan_sku : planSku,
		link_token: typeof body.link_token === 'string' ? body.link_token : ''
	};
}
