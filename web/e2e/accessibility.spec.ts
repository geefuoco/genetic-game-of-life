import { expect, test, type Page } from '@playwright/test';
import { accessibilityViolations } from './axe.ts';
import { stats } from './stats.ts';

async function open(page: Page): Promise<void> {
  await page.goto('./');
  await expect.poll(async () => (await stats(page)).generation).toBeGreaterThan(2);
}

test('the main view passes an automated accessibility check', async ({ page }) => {
  await open(page);
  expect(await accessibilityViolations(page)).toEqual([]);
});

test('the main view passes an automated accessibility check with the controls docked to the side', async ({ page }) => {
  await open(page);
  await page.getByRole('button', { name: 'Dock the controls to the side' }).click();
  await expect(page.getByRole('complementary', { name: 'Side panel' })).toBeVisible();
  expect(await accessibilityViolations(page)).toEqual([]);
});
