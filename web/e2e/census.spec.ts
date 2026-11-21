import { expect, test, type Locator, type Page } from '@playwright/test';
import { stats } from './stats.ts';
import { dragRow, openPaused } from './world.ts';

const statsRegion = (page: Page): Locator => page.getByRole('region', { name: 'Stats' });

/** The number the stats panel shows beside `label`. */
async function figure(region: Locator, label: string): Promise<number> {
  const text = (await region.locator(`dt:text-is("${label}") + dd`).textContent()) ?? '';
  return Number(text.replaceAll(',', ''));
}

test('the stats panel is closed by default, and opens from the keyboard with the Generation, Population and living colour genes as the run goes', async ({ page }) => {
  await page.goto('./');
  await expect.poll(async () => (await stats(page)).generation).toBeGreaterThan(2);
  await expect(statsRegion(page)).toBeHidden();

  await page.keyboard.press('d');
  const region = statsRegion(page);
  await expect(region).toBeVisible();
  const opened = await figure(region, 'generation');
  await expect.poll(() => figure(region, 'generation')).toBeGreaterThan(opened + 5);
  expect(await figure(region, 'colour genes')).toBeGreaterThan(0);
  await expect(region.getByRole('list', { name: 'Colour genes' }).getByRole('listitem').first()).toHaveAccessibleName(/^\w+: carried by [\d,]+, (shown by [\d,]+|hidden everywhere)$/);
  await expect(region.getByRole('img', { name: /^Population over time/ })).toBeVisible();

  await page.keyboard.press('Space');
  await page.waitForTimeout(500);
  const line = await stats(page);
  expect(await figure(region, 'generation')).toBe(line.generation);
  expect(await figure(region, 'alive')).toBe(line.population);

  await page.keyboard.press('d');
  await expect(statsRegion(page)).toBeHidden();
});

test('compares how many Cells carry and show each colour gene, marking one every Carrier hides', async ({ page }) => {
  await openPaused(page);
  await page.keyboard.press('d');
  const panel = page.getByRole('complementary', { name: 'Side panel' });
  await page.waitForTimeout(500);
  await panel.getByRole('slider', { name: 'Density' }).focus();
  await page.keyboard.press('Home');
  await panel.getByRole('slider', { name: 'Colour genes' }).focus();
  await page.keyboard.press('Home');
  await panel.getByRole('button', { name: 'Start new soup' }).click();
  await expect.poll(async () => (await stats(page)).generation).toBe(0);

  await panel.getByRole('button', { name: 'Erase' }).click();
  await dragRow(page, 80, 70, 90, 'docked');
  await panel.getByRole('button', { name: 'Carrier brush' }).click();
  await dragRow(page, 80, 72, 81, 'docked');

  const region = statsRegion(page);
  const genes = region.getByRole('list', { name: 'Colour genes' });
  await expect(genes.getByRole('listitem', { name: 'Red: carried by 10, shown by 10' })).toBeVisible();
  await expect(genes.getByRole('listitem', { name: 'Sky: carried by 10, hidden everywhere' })).toBeVisible();
  expect(await figure(region, 'colour genes')).toBe(3);
  expect(await figure(region, 'hidden everywhere')).toBe(1);
});

test('the population chart can be read from the keyboard', async ({ page }) => {
  await openPaused(page);
  await page.keyboard.press('d');
  await statsRegion(page).getByRole('slider', { name: 'Population at a generation' }).focus();
  await page.keyboard.press('Home');
  await expect(statsRegion(page).getByTestId('population-readout')).toHaveText(/^[\d,]+ alive · gen 0$/);
  await page.keyboard.press('End');
  const line = await stats(page);
  await expect(statsRegion(page).getByTestId('population-readout')).toHaveText(`${line.population.toLocaleString('en-US')} alive · gen ${line.generation.toLocaleString('en-US')}`);
});
