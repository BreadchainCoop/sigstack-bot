import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test.describe('smoke', () => {
	test('home loads with brand and path chooser', async ({ page }) => {
		await page.goto('./');
		await expect(page.getByRole('heading', { level: 1, name: 'Bread Bot' })).toBeVisible();
		await expect(page.getByRole('heading', { name: 'Which mode do you want?' })).toBeVisible();
		await expect(page.getByRole('link', { name: /Learn more/i }).first()).toBeVisible();
	});

	test('product pages and diagrams', async ({ page }) => {
		for (const path of ['./language-threads/', './in-chat/', './transcription/']) {
			await page.goto(path);
			const img = page.locator('.diagram-frame img');
			await expect(img).toBeVisible();
			await expect(img).toHaveJSProperty('complete', true);
			expect(await img.evaluate((el) => (el as HTMLImageElement).clientHeight)).toBeGreaterThan(50);
		}
	});

	test('primary nav reaches FAQ', async ({ page }) => {
		await page.goto('./');
		await page.setViewportSize({ width: 1200, height: 800 });
		await page.getByRole('navigation', { name: 'Primary' }).getByRole('link', { name: 'FAQ' }).click();
		await expect(page.getByRole('heading', { level: 1, name: 'FAQ' })).toBeVisible();
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
