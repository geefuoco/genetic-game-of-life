import { expect, test, type Locator, type Page } from '@playwright/test';

async function stats(page: Page): Promise<{ generation: number; population: number }> {
  const text = (await page.getByTestId('stats').textContent()) ?? '';
  const match = /gen ([\d,]+) · ([\d,]+) alive/.exec(text);
  if (!match) throw new Error(`unexpected stats line: ${text}`);
  const number = (digits: string | undefined): number => Number((digits ?? '').replaceAll(',', ''));
  return { generation: number(match[1]), population: number(match[2]) };
}

/** Opens the page, waits for it to run, then pauses and waits for any frame in flight. */
async function openPaused(page: Page): Promise<void> {
  await page.goto('./');
  await expect.poll(async () => (await stats(page)).generation).toBeGreaterThan(2);
  await page.getByRole('button', { name: 'Pause' }).click();
  await page.waitForTimeout(300);
}

async function openSettings(page: Page): Promise<Locator> {
  await page.getByRole('button', { name: 'Settings' }).click();
  const settings = page.getByRole('dialog', { name: 'Settings' });
  await expect(settings).toBeVisible();
  return settings;
}

/** Steps `count` Generations, returning the population after each. */
async function stepPopulations(page: Page, count: number): Promise<number[]> {
  const populations: number[] = [];
  for (let i = 0; i < count; i++) {
    const before = (await stats(page)).generation;
    await page.getByRole('button', { name: 'Step one generation' }).click();
    await expect.poll(async () => (await stats(page)).generation).toBe(before + 1);
    populations.push((await stats(page)).population);
  }
  return populations;
}

const isNonDecreasing = (values: number[]): boolean => values.every((value, i) => i === 0 || value >= (values[i - 1] ?? value));

test('changes the Rule mid-run from a Rule Preset or custom notation, keeping it when the notation is invalid', async ({ page }) => {
  await openPaused(page);
  const before = await stats(page);
  const settings = await openSettings(page);
  const preset = settings.getByRole('combobox', { name: 'Rule', exact: true });
  const custom = settings.getByRole('textbox', { name: 'Custom rule' });
  await expect(preset).toHaveValue('B3/S23');

  await preset.selectOption({ label: 'Life without Death' });
  await expect(custom).toHaveValue('B3/S012345678');
  const populations = await stepPopulations(page, 4);
  expect((await stats(page)).generation).toBe(before.generation + 4);
  expect(isNonDecreasing([before.population, ...populations])).toBe(true);

  await custom.fill('B3/S2x');
  await custom.press('Enter');
  await expect(settings.getByRole('alert')).toHaveText("'x' is not a digit from 0 to 8");
  await expect(custom).toHaveAttribute('aria-invalid', 'true');
  await expect(preset).toHaveValue('B3/S012345678');
  const after = (await stats(page)).population;
  expect(isNonDecreasing([after, ...(await stepPopulations(page, 3))])).toBe(true);

  await custom.fill('b63/s32');
  await custom.press('Enter');
  await expect(preset).toHaveValue('B36/S23');
  await expect(custom).toHaveValue('B36/S23');
  await expect(settings.getByRole('alert')).toHaveCount(0);
});

test('starts a new Soup with the chosen size, density, colour genes and Seed', async ({ page }) => {
  await openPaused(page);
  const settings = await openSettings(page);
  const width = settings.getByRole('spinbutton', { name: 'Width' });
  const height = settings.getByRole('spinbutton', { name: 'Height' });
  const density = settings.getByRole('slider', { name: 'Density' });
  const genes = settings.getByRole('slider', { name: 'Colour genes' });
  const seed = settings.getByRole('textbox', { name: 'Seed' });
  const start = settings.getByRole('button', { name: 'Start new soup' });

  await width.fill('64');
  await height.fill('32');
  await density.focus();
  await page.keyboard.press('End');
  await expect(settings.getByText('100%')).toBeVisible();
  await genes.focus();
  await page.keyboard.press('Home');
  await page.keyboard.press('ArrowRight');
  await page.keyboard.press('ArrowRight');
  await expect(genes).toHaveValue('3');
  await start.click();
  await expect(page.getByTestId('stats')).toHaveText('gen 0 · 2,048 alive');

  await settings.getByRole('button', { name: '128 by 128' }).click();
  await expect(width).toHaveValue('128');
  await expect(height).toHaveValue('128');
  await start.click();
  await expect(page.getByTestId('stats')).toHaveText('gen 0 · 16,384 alive');

  await density.focus();
  await page.keyboard.press('Home');
  for (let i = 0; i < 39; i++) await page.keyboard.press('ArrowRight');
  await expect(density).toHaveAttribute('aria-valuetext', '40%');
  await seed.fill('42');
  await start.click();
  await expect.poll(async () => (await stats(page)).population).not.toBe(16384);
  const seeded = await stats(page);
  expect(seeded.generation).toBe(0);
  await page.getByRole('button', { name: 'Step one generation' }).click();
  await expect.poll(async () => (await stats(page)).generation).toBe(1);
  await start.click();
  await expect(page.getByTestId('stats')).toHaveText(`gen 0 · ${seeded.population.toLocaleString('en-US')} alive`);
});

test('explains an invalid New Soup form and keeps the running World', async ({ page }) => {
  await openPaused(page);
  const before = await stats(page);
  const settings = await openSettings(page);
  await settings.getByRole('spinbutton', { name: 'Width' }).fill('8');
  await settings.getByRole('textbox', { name: 'Seed' }).fill('-1');
  await settings.getByRole('button', { name: 'Start new soup' }).click();
  await expect(settings.getByText('Width must be a whole number from 16 to 1024.')).toBeVisible();
  await expect(settings.getByRole('spinbutton', { name: 'Width' })).toHaveAccessibleDescription('Width must be a whole number from 16 to 1024.');
  await expect(settings.getByText(/^Seed must be a whole number/)).toBeVisible();
  await page.waitForTimeout(300);
  expect(await stats(page)).toEqual(before);
});

test('changes the Mutation settings, showing each value beside its control', async ({ page }) => {
  await openPaused(page);
  const settings = await openSettings(page);
  const rate = settings.getByRole('slider', { name: 'Mutation', exact: true });
  const reach = settings.getByRole('slider', { name: 'Mutation reach' });
  await expect(rate).toHaveAttribute('aria-valuetext', '1.0%');
  await expect(reach).toHaveAttribute('aria-valuetext', '±30°');
  await rate.focus();
  await page.keyboard.press('ArrowRight');
  await page.keyboard.press('ArrowRight');
  await expect(rate).toHaveAttribute('aria-valuetext', '1.2%');
  await expect(settings.getByText('1.2%')).toBeVisible();
  await reach.focus();
  await page.keyboard.press('End');
  await expect(settings.getByText('±90°')).toBeVisible();
});

test('settings open from the keyboard, every control in them is labelled, and Escape closes them', async ({ page }) => {
  await openPaused(page);
  await page.getByRole('button', { name: 'Settings' }).focus();
  await page.keyboard.press('Enter');
  const settings = page.getByRole('dialog', { name: 'Settings' });
  await expect(settings.getByRole('combobox', { name: 'Rule', exact: true })).toBeFocused();
  const names: string[] = [];
  for (;;) {
    const focused = page.locator(':focus');
    if ((await settings.locator(':focus').count()) === 0) break;
    await expect(focused).toHaveAccessibleName(/\S/);
    names.push(await focused.evaluate((element) => element.id || element.textContent || ''));
    await page.keyboard.press('Tab');
  }
  expect(names.length).toBeGreaterThanOrEqual(14);
  await settings.getByRole('textbox', { name: 'Seed' }).focus();
  await page.keyboard.press('Escape');
  await expect(settings).toBeHidden();
  await expect(page.getByRole('button', { name: 'Settings' })).toBeFocused();
});

test('the docked panel holds the same settings in their own section', async ({ page }) => {
  await openPaused(page);
  await page.getByRole('button', { name: 'Dock the controls to the side' }).click();
  const panel = page.getByRole('complementary', { name: 'Side panel' });
  const section = panel.getByRole('region', { name: 'Settings' });
  await expect(panel.getByRole('button', { name: 'Settings' })).toHaveCount(0);
  await section.getByRole('combobox', { name: 'Rule', exact: true }).selectOption({ label: 'HighLife' });
  await expect(section.getByRole('textbox', { name: 'Custom rule' })).toHaveValue('B36/S23');
  await expect(section.getByRole('slider', { name: 'Mutation', exact: true })).toBeVisible();
  await expect(section.getByRole('button', { name: 'Start new soup' })).toBeVisible();
});
