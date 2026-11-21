import { expect, test, type Page } from '@playwright/test';
import { stats } from './stats.ts';
import { centreOf, dragRow, openPaused } from './world.ts';

/** Clears rows 76 to 84 around the middle, then paints one lone Carrier of the default brush at (80, 80). */
async function paintLoneCarrier(page: Page): Promise<void> {
  const tools = page.getByRole('group', { name: 'Tools' });
  await tools.getByRole('button', { name: 'Erase' }).click();
  for (let y = 76; y <= 84; y++) await dragRow(page, y, 70, 90);
  await tools.getByRole('button', { name: 'Carrier brush' }).click();
  const cell = centreOf(80, 80);
  await page.mouse.click(cell.x, cell.y);
  await tools.getByRole('button', { name: 'Look' }).click();
}

test('clicking an Organism shows its Genotype, and the inspector stays with it through its death', async ({ page }) => {
  await openPaused(page);
  await paintLoneCarrier(page);
  const painted = (await stats(page)).generation;

  const cell = centreOf(80, 80);
  await page.mouse.click(cell.x, cell.y);
  const inspector = page.getByRole('region', { name: 'Red cell' });
  await expect(inspector).toBeVisible();
  await expect(inspector.getByText('Carries a hidden colour')).toBeVisible();
  const genes = inspector.getByRole('group', { name: 'Its colour genes' });
  await expect(genes).toContainText(/red\s*shown/i);
  await expect(genes).toContainText(/sky\s*hidden/i);
  await expect(genes.getByRole('img', { name: 'Strength 3 of 4, strong' })).toBeVisible();
  await expect(genes.getByRole('img', { name: 'Strength 1 of 4, weakest' })).toBeVisible();
  await expect(inspector.getByRole('list', { name: 'Lineage of the red colour gene, oldest first' })).toContainText(
    `added gen ${painted.toLocaleString()}`,
  );
  await expect(inspector).toContainText(`Born at generation ${painted.toLocaleString()}`);

  await page.getByRole('button', { name: 'Step one generation' }).click();
  await expect(inspector.getByText(`Died at generation ${(painted + 1).toLocaleString()}`)).toBeVisible();
  await expect(inspector.getByRole('group', { name: 'Its colour genes' })).toContainText(/red\s*shown/i);
});

test('the inspector is reached with Tab and closed with its button or Escape', async ({ page }) => {
  await openPaused(page);
  await paintLoneCarrier(page);
  const cell = centreOf(80, 80);
  await page.mouse.click(cell.x, cell.y);
  const inspector = page.getByRole('region', { name: 'Red cell' });
  await expect(inspector).toBeVisible();

  const close = inspector.getByRole('button', { name: 'Close the inspector' });
  await page.keyboard.press('Tab');
  await expect(close).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(inspector).toBeHidden();

  await page.mouse.click(cell.x, cell.y);
  await expect(inspector).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(inspector).toBeHidden();
});

test('clicking a dead Cell inspects nothing, and clicking outside the World closes the inspector', async ({ page }) => {
  await openPaused(page);
  await paintLoneCarrier(page);
  const dead = centreOf(84, 84);
  await page.mouse.click(dead.x, dead.y);
  await page.waitForTimeout(300);
  await expect(page.getByRole('region', { name: /cell$/ })).toHaveCount(0);

  const cell = centreOf(80, 80);
  await page.mouse.click(cell.x, cell.y);
  const inspector = page.getByRole('region', { name: 'Red cell' });
  await expect(inspector).toBeVisible();
  const zoomOut = page.getByRole('button', { name: 'Zoom out' });
  for (let i = 0; i < 12; i++) await zoomOut.click();
  await page.mouse.click(60, 450);
  await expect(inspector).toBeHidden();
});

test("a painted Organism's Pedigree shows it has no Parents", async ({ page }) => {
  await openPaused(page);
  await paintLoneCarrier(page);
  const painted = (await stats(page)).generation;
  const cell = centreOf(80, 80);
  await page.mouse.click(cell.x, cell.y);
  const ancestors = page.getByRole('region', { name: 'Red cell' }).getByRole('region', { name: 'Ancestors' });
  await expect(ancestors.getByRole('img', { name: `Painted or seeded at gen ${painted.toLocaleString()}, no parents` })).toBeVisible();
  await expect(ancestors.getByRole('button', { name: 'Show more ancestors' })).toHaveCount(0);
});

/** Clears rows 70 to 90 around the middle, then paints a row of three Carriers at (79..81, 80), a blinker. */
async function paintBlinker(page: Page): Promise<void> {
  const tools = page.getByRole('group', { name: 'Tools' });
  await tools.getByRole('button', { name: 'Erase' }).click();
  for (let y = 70; y <= 90; y++) await dragRow(page, y, 66, 94);
  await tools.getByRole('button', { name: 'Carrier brush' }).click();
  await dragRow(page, 80, 79, 81);
  await tools.getByRole('button', { name: 'Look' }).click();
}

async function step(page: Page, count: number): Promise<void> {
  for (let i = 0; i < count; i++) {
    const before = (await stats(page)).generation;
    await page.getByRole('button', { name: 'Step one generation' }).click();
    await expect.poll(async () => (await stats(page)).generation).toBe(before + 1);
  }
}

test("a Born Organism's Pedigree shows its Parents, and older ancestors on request", async ({ page }) => {
  await openPaused(page);
  await paintBlinker(page);
  const painted = (await stats(page)).generation;
  await step(page, 1);

  const top = centreOf(80, 79);
  await page.mouse.click(top.x, top.y);
  const ancestors = page.getByRole('region', { name: /cell$/ }).getByRole('region', { name: 'Ancestors' });
  await expect(ancestors.getByRole('img', { name: `Born at gen ${(painted + 1).toLocaleString()} from 2 parents` })).toBeVisible();
  const parents = ancestors.getByRole('group', { name: 'Parents' });
  await expect(parents.getByRole('img', { name: `Painted or seeded at gen ${painted.toLocaleString()}, no parents` })).toHaveCount(2);

  await step(page, 6);
  await page.mouse.click(top.x, top.y);
  await expect(ancestors.getByRole('img', { name: `Born at gen ${(painted + 7).toLocaleString()} from 2 parents` })).toBeVisible();
  await expect(ancestors.getByRole('group', { name: 'Great-grandparents' })).toBeVisible();
  await expect(ancestors.getByRole('group', { name: 'Great-great-grandparents' })).toHaveCount(0);
  await ancestors.getByRole('button', { name: 'Show more ancestors' }).click();
  await expect(ancestors.getByRole('group', { name: 'Great-great-grandparents' })).toBeVisible();
});
