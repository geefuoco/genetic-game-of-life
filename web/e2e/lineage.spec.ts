import { expect, test, type Page } from '@playwright/test';
import { stats } from './stats.ts';
import { centreOf, dragRow, openPaused } from './world.ts';

const DESCRIBED = /^\w+: (A founding colour gene|Mutated from \w+ at gen [\d,]+|Added at gen [\d,]+)\. ([\d,]+% of colour genes, (shown by [\d,]+ cells|hidden everywhere)|Died out after gen [\d,]+)\.( Highlighted\.)?$/;

const highlight = (page: Page) => page.getByRole('region', { name: 'Highlight' });

/**
 * A paused Soup from a fixed Seed, stepped `generations` times, so its
 * Allele Tree is the same on every run and no Allele is pruned mid-test.
 */
async function seededSoup(page: Page, generations: number): Promise<void> {
  await openPaused(page);
  await page.getByRole('button', { name: 'Settings' }).click();
  const settings = page.getByRole('dialog', { name: 'Settings' });
  await settings.getByRole('textbox', { name: 'Seed' }).fill('42');
  await settings.getByRole('button', { name: 'Start new soup' }).click();
  await expect.poll(async () => (await stats(page)).generation).toBe(0);
  await page.keyboard.press('Escape');
  await expect(settings).toHaveCount(0);
  const step = page.getByRole('button', { name: 'Step one generation' });
  for (let i = 0; i < generations; i++) await step.click();
  await expect.poll(async () => (await stats(page)).generation).toBe(generations);
}

test('the Lineage card opens from the dock, and the keyboard reads its colour genes and lights up one of them until Escape', async ({ page }) => {
  await seededSoup(page, 30);
  const toggle = page.getByRole('button', { name: 'Lineage' });
  await expect(toggle).toHaveAttribute('aria-expanded', 'false');
  await toggle.click();
  await expect(toggle).toHaveAttribute('aria-expanded', 'true');
  const lineage = page.getByRole('figure', { name: 'Lineage' });
  await expect(lineage.getByTestId('lineage-summary')).toHaveText(/^Colour gene lineages from gen 0 to gen [\d,]+: [\d,]+ shown, [\d,]+ hidden everywhere, [\d,]+ died out$/);

  const reader = lineage.getByRole('slider', { name: 'Colour gene lineages, by hue' });
  await reader.focus();
  await page.keyboard.press('ArrowRight');
  await expect(reader).toHaveAttribute('aria-valuetext', DESCRIBED);
  const name = /^(\w+):/.exec((await reader.getAttribute('aria-valuetext')) ?? '')?.[1];
  await page.keyboard.press('Enter');
  await expect(highlight(page)).toContainText(`${name} colour gene`);
  await expect(highlight(page)).toContainText(/[\d,]+ show it\s*[\d,]+ carry it hidden|carried by none/);
  await expect(reader).toHaveAttribute('aria-valuetext', /Highlighted\.$/);

  await page.keyboard.press('Escape');
  await expect(highlight(page)).toHaveCount(0);

  await toggle.click();
  await expect(lineage).toHaveCount(0);
});

test('the docked side panel shows the Lineage', async ({ page }) => {
  await page.goto('./');
  await page.keyboard.press('d');
  const panel = page.getByRole('complementary', { name: 'Side panel' });
  await expect(panel.getByRole('figure', { name: 'Lineage' }).getByTestId('lineage-summary')).toHaveText(/^Colour gene lineages/);
});

test('lighting up a hidden colour gene from the inspector counts its Carriers apart from the Cells that show it', async ({ page }) => {
  await openPaused(page);
  const tools = page.getByRole('group', { name: 'Tools' });
  await tools.getByRole('button', { name: 'Erase' }).click();
  for (let y = 76; y <= 84; y++) await dragRow(page, y, 70, 90);
  await tools.getByRole('button', { name: 'Carrier brush' }).click();
  await dragRow(page, 80, 75, 85);
  await tools.getByRole('button', { name: 'Look' }).click();
  const cell = centreOf(80, 80);
  await page.mouse.click(cell.x, cell.y);

  const gene = page.getByRole('region', { name: 'Red cell' }).getByRole('button', { name: 'Light up the cells carrying the hidden sky colour gene' });
  await gene.click();
  await expect(gene).toHaveAttribute('aria-pressed', 'true');
  await expect(highlight(page)).toContainText('Sky colour gene');
  await expect(highlight(page)).toContainText(/0 show it\s*11 carry it hidden/);

  await page.getByRole('button', { name: 'Step one generation' }).click();
  await expect(highlight(page)).not.toContainText('11 carry it hidden');
  await expect(highlight(page)).toContainText(/[\d,]+ show it\s*[\d,]+ carry it hidden/);

  await gene.click();
  await expect(highlight(page)).toHaveCount(0);
});
