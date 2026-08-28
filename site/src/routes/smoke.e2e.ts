import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test.describe('smoke', () => {
	test('home loads with brand stub and CTAs', async ({ page }) => {
		await page.goto('./');
		await expect(page.getByRole('heading', { level: 1, name: 'Bread Bot' })).toBeVisible();
		await expect(page.getByText('Placeholder — redesign pending.')).toBeVisible();
		await expect(page.getByRole('link', { name: 'Get started' }).first()).toBeVisible();
		await expect(page.getByRole('link', { name: 'Products' }).first()).toBeVisible();
	});

	test('products dropdown pins language threads section', async ({ page }) => {
		await page.goto('./');
		await page.setViewportSize({ width: 1200, height: 800 });
		const nav = page.getByRole('navigation', { name: 'Primary' });
		await nav.getByRole('button', { name: 'Products menu' }).click();
		await nav.getByRole('link', { name: 'Language Threads' }).click();
		await expect(page.getByRole('heading', { level: 2, name: 'Language Threads' })).toBeVisible();
		await expect(page).toHaveURL(/#language-threads/);
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

	test('products page has no serious a11y violations', async ({ page }) => {
		await page.goto('./products/');
		const results = await new AxeBuilder({ page }).analyze();
		const serious = results.violations.filter((v) =>
			['serious', 'critical'].includes(v.impact ?? '')
		);
		expect(serious).toEqual([]);
	});
});
