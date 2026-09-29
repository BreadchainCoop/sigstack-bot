import { afterEach, describe, expect, it, vi } from 'vitest';
import { commerceApiBase, createCheckoutSession } from './commerceCheckout';

describe('commerceApiBase', () => {
	it('returns null for missing or blank', () => {
		expect(commerceApiBase(undefined)).toBeNull();
		expect(commerceApiBase(null)).toBeNull();
		expect(commerceApiBase('')).toBeNull();
		expect(commerceApiBase('   ')).toBeNull();
	});

	it('trims and strips trailing slashes', () => {
		expect(commerceApiBase(' http://localhost:8082/ ')).toBe('http://localhost:8082');
		expect(commerceApiBase('https://example.com/commerce///')).toBe('https://example.com/commerce');
	});
});

describe('createCheckoutSession', () => {
	afterEach(() => {
		vi.restoreAllMocks();
	});

	it('fails when base URL is blank', async () => {
		const fetchImpl = vi.fn();
		await expect(createCheckoutSession('  ', 'all-access-3', fetchImpl)).resolves.toEqual({
			ok: false,
			error: 'checkout API base URL is not configured'
		});
		expect(fetchImpl).not.toHaveBeenCalled();
	});

	it('POSTs plan_sku and returns checkout url', async () => {
		const fetchImpl = vi.fn().mockResolvedValue({
			ok: true,
			status: 200,
			json: async () => ({
				url: 'https://checkout.stripe.com/c/pay/cs_test_1',
				plan_sku: 'all-access-3',
				link_token: 'tok-abc'
			})
		});

		const result = await createCheckoutSession(
			'http://localhost:8082/',
			'all-access-3',
			fetchImpl
		);

		expect(result).toEqual({
			ok: true,
			url: 'https://checkout.stripe.com/c/pay/cs_test_1',
			plan_sku: 'all-access-3',
			link_token: 'tok-abc'
		});
		expect(fetchImpl).toHaveBeenCalledWith('http://localhost:8082/v1/checkout/sessions', {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				Accept: 'application/json'
			},
			body: JSON.stringify({ plan_sku: 'all-access-3' })
		});
	});

	it('surfaces API error body', async () => {
		const fetchImpl = vi.fn().mockResolvedValue({
			ok: false,
			status: 503,
			json: async () => ({ error: 'price id not configured' })
		});

		await expect(
			createCheckoutSession('http://localhost:8082', 'all-access-10', fetchImpl)
		).resolves.toEqual({
			ok: false,
			error: 'price id not configured'
		});
	});

	it('returns network error when fetch throws', async () => {
		const fetchImpl = vi.fn().mockRejectedValue(new TypeError('Failed to fetch'));
		await expect(
			createCheckoutSession('http://localhost:8082', 'all-access-3', fetchImpl)
		).resolves.toEqual({
			ok: false,
			error: 'network error starting checkout'
		});
	});

	it('fails when success body has no url', async () => {
		const fetchImpl = vi.fn().mockResolvedValue({
			ok: true,
			status: 200,
			json: async () => ({ plan_sku: 'all-access-3', link_token: 'x' })
		});
		await expect(
			createCheckoutSession('http://localhost:8082', 'all-access-3', fetchImpl)
		).resolves.toEqual({
			ok: false,
			error: 'checkout response missing url'
		});
	});
});
