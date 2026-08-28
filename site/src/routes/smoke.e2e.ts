import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test.describe('smoke', () => {
	test('home loads with brand stub', async ({ page }) => {
		await page.goto('./');
		await expect(page.getByRole('heading', { level: 1, name: 'Bread Bot' })).toBeVisible();
		await expect(page.getByText('Placeholder — redesign pending.')).toBeVisible();
	});

	test('primary nav reaches product stubs', async ({ page }) => {
		await page.goto('./');
		await page.setViewportSize({ width: 1200, height: 800 });
		const nav = page.getByRole('navigation', { name: 'Primary' });
		await nav.getByRole('link', { name: 'Suite' }).click();
		await expect(page.getByRole('heading', { level: 1, name: 'Product suite' })).toBeVisible();
		await nav.getByRole('link', { name: 'Language Threads' }).click();
		await expect(page.getByRole('heading', { level: 1, name: 'Language Threads' })).toBeVisible();
		await expect(nav.getByRole('link', { name: 'FAQ' })).toHaveCount(0);
	});

	test('theme toggle flips data-theme', async ({ page }) => {
		await page.goto('./');
		await page.setViewportSize({ width: 1200, height: 800 });
		const html = page.locator('html');
		const initial = await html.getAttribute('data-theme');
		expect(initial === 'light' || initial === 'dark').toBe(true);
		const next = initial === 'dark' ? 'light' : 'dark';
		const label = initial === 'dark' ? 'Switch to light mode' : 'Switch to dark mode';
		await page.getByRole('button', { name: label }).click();
		await expect(html).toHaveAttribute('data-theme', next);
		const restoreLabel = next === 'dark' ? 'Switch to light mode' : 'Switch to dark mode';
		await page.getByRole('button', { name: restoreLabel }).click();
		await expect(html).toHaveAttribute('data-theme', initial!);
	});

	test('landing has no serious a11y violations', async ({ page }) => {
		await page.goto('./');
		const results = await new AxeBuilder({ page }).analyze();
		const serious = results.violations.filter((v) =>
			['serious', 'critical'].includes(v.impact ?? '')
		);
		expect(serious).toEqual([]);
	});

	test('language-threads has no serious a11y violations', async ({ page }) => {
		await page.goto('./language-threads/');
		const results = await new AxeBuilder({ page }).analyze();
		const serious = results.violations.filter((v) =>
			['serious', 'critical'].includes(v.impact ?? '')
		);
		expect(serious).toEqual([]);
	});
});
