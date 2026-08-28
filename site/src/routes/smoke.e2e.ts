import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

async function expectNoHorizontalOverflow(page: import('@playwright/test').Page) {
	const overflow = await page.evaluate(
		() => document.documentElement.scrollWidth > window.innerWidth + 1
	);
	expect(overflow).toBe(false);
}

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

	test('privacy page loads trust sections', async ({ page }) => {
		await page.goto('./privacy/');
		await expect(page.getByRole('heading', { level: 1, name: 'Privacy and trust' })).toBeVisible();
		await expect(page.getByRole('heading', { level: 2, name: 'What a TEE is' })).toBeVisible();
		await expect(page.getByRole('heading', { level: 2, name: 'Promise vs proof' })).toBeVisible();
		await expect(page.getByRole('link', { name: 'Privacy Policy' }).first()).toBeVisible();
	});

	test('footer Legal links to Apache license page', async ({ page }) => {
		await page.goto('./');
		const legal = page.getByRole('contentinfo').getByRole('link', { name: 'Legal' });
		await expect(legal).toBeVisible();
		await legal.click();
		await expect(page).toHaveURL(/\/license\/?/);
		await expect(page.getByRole('heading', { level: 1, name: 'Apache License 2.0' })).toBeVisible();
		await expect(page.getByText('Copyright 2024 Zaki Manian', { exact: true })).toBeVisible();
	});

	test('privacy page has no serious a11y violations', async ({ page }) => {
		await page.goto('./privacy/');
		const results = await new AxeBuilder({ page }).analyze();
		const serious = results.violations.filter((v) =>
			['serious', 'critical'].includes(v.impact ?? '')
		);
		expect(serious).toEqual([]);
	});

	test('home, products, and privacy have no horizontal overflow at desktop', async ({ page }) => {
		await page.setViewportSize({ width: 1200, height: 800 });
		await page.goto('./');
		await expectNoHorizontalOverflow(page);
		await page.goto('./products/');
		await expectNoHorizontalOverflow(page);
		await page.goto('./privacy/');
		await expectNoHorizontalOverflow(page);
	});
});

test.describe('smoke mobile', () => {
	test.use({ viewport: { width: 390, height: 844 } });

	test('drawer opens and products anchors navigate', async ({ page }) => {
		await page.goto('./');
		await expect(page.getByRole('heading', { level: 1, name: 'Bread Bot' })).toBeVisible();
		await expectNoHorizontalOverflow(page);

		await page.getByRole('button', { name: 'Open menu' }).click();
		const nav = page.getByRole('navigation', { name: 'Primary' });
		await expect(nav).toBeVisible();
		await nav.getByRole('link', { name: 'Language Threads' }).click();
		await expect(page.getByRole('heading', { level: 2, name: 'Language Threads' })).toBeVisible();
		await expect(page).toHaveURL(/#language-threads/);
	});

	test('theme toggle works from mobile drawer', async ({ page }) => {
		await page.goto('./');
		await page.getByRole('button', { name: 'Open menu' }).click();
		const html = page.locator('html');
		const initial = await html.getAttribute('data-theme');
		expect(initial === 'light' || initial === 'dark').toBe(true);
		const next = initial === 'dark' ? 'light' : 'dark';
		const label = initial === 'dark' ? 'Switch to light mode' : 'Switch to dark mode';
		await page.getByRole('button', { name: label }).click();
		await expect(html).toHaveAttribute('data-theme', next);
	});

	test('products page shows stacked diagram and command list without overflow', async ({
		page
	}) => {
		await page.goto('./products/');
		await expect(page.getByRole('heading', { level: 2, name: 'Language Threads' })).toBeVisible();
		await expect(page.locator('.layout-narrow')).toBeVisible();
		await expect(page.locator('.layout-wide')).toBeHidden();
		await expectNoHorizontalOverflow(page);

		await page.getByRole('button', { name: 'command list' }).first().click();
		await expect(page.getByRole('region', { name: 'command list' }).first()).toBeVisible();
		await expectNoHorizontalOverflow(page);

		const results = await new AxeBuilder({ page }).analyze();
		const serious = results.violations.filter((v) =>
			['serious', 'critical'].includes(v.impact ?? '')
		);
		expect(serious).toEqual([]);
	});
});
