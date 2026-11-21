import { expect, test, type Page } from '@playwright/test';
import { DEFAULT_BRUSH } from '../src/lib/brush.ts';
import { paletteCss } from '../src/lib/palette.ts';
import { stats } from './stats.ts';
import { centreOf, dragRow, openPaused, population } from './world.ts';

/** Expects the screen to show the palette colour of `hue` at `point`. */
async function expectColourAt(page: Page, point: { x: number; y: number }, hue: number): Promise<void> {
  const expected = /rgb\((\d+) (\d+) (\d+)\)/.exec(paletteCss(hue))?.slice(1).map(Number) ?? [];
  const shown = await pixelAt(page, point);
  shown.forEach((channel, i) => expect(Math.abs(channel - (expected[i] ?? Number.NaN))).toBeLessThanOrEqual(8));
}

async function pixelAt(page: Page, point: { x: number; y: number }): Promise<number[]> {
  const png = (await page.screenshot({ clip: { x: point.x, y: point.y, width: 1, height: 1 } })).toString('base64');
  return page.evaluate(async (base64) => {
    const bytes = Uint8Array.from(atob(base64), (c) => c.charCodeAt(0));
    const bitmap = await createImageBitmap(new Blob([bytes], { type: 'image/png' }));
    const canvas = new OffscreenCanvas(1, 1);
    const context = canvas.getContext('2d');
    if (!context) throw new Error('no 2D context');
    context.drawImage(bitmap, 0, 0);
    return Array.from(context.getImageData(0, 0, 1, 1).data.slice(0, 3));
  }, png);
}

test('paints live Cells that appear in the World, erases, stamps, and undoes and redoes Strokes', async ({ page }) => {
  await openPaused(page);
  const dock = page.getByRole('group', { name: 'Tools' });
  const undo = page.getByRole('button', { name: 'Undo' });
  const redo = page.getByRole('button', { name: 'Redo' });
  await expect(undo).toBeDisabled();

  await dock.getByRole('button', { name: 'Erase' }).click();
  await expect(dock.getByRole('button', { name: 'Erase' })).toHaveAttribute('aria-pressed', 'true');
  for (let y = 76; y <= 84; y++) await dragRow(page, y, 60, 100);
  await expect(undo).toBeEnabled();
  await page.waitForTimeout(200);
  const erased = await population(page);

  await dock.getByRole('button', { name: 'Paint' }).click();
  await dragRow(page, 80, 62, 81);
  await expect.poll(() => population(page)).toBe(erased + 20);
  await expectColourAt(page, centreOf(70, 80), DEFAULT_BRUSH.first.hue);

  await page.keyboard.press('Control+z');
  await expect.poll(() => population(page)).toBe(erased);
  await expect(redo).toBeEnabled();
  await redo.click();
  await expect.poll(() => population(page)).toBe(erased + 20);
  await expect(redo).toBeDisabled();

  await dock.getByRole('button', { name: 'Stamp a pattern' }).click();
  const glider = centreOf(90, 78);
  await page.mouse.click(glider.x, glider.y);
  await expect.poll(() => population(page)).toBe(erased + 25);

  await page.getByRole('button', { name: 'Step one generation' }).click();
  await expect.poll(async () => (await stats(page)).generation).toBeGreaterThan(0);
  await expect(undo).toBeDisabled();
});

test('in the docked layout, the Carrier brush paints Carriers that show the dominant colour, and Ctrl+Z works from a brush slider', async ({ page }) => {
  await openPaused(page);
  await page.getByRole('button', { name: 'Dock the controls to the side' }).click();
  const panel = page.getByRole('complementary', { name: 'Side panel' });
  await page.waitForTimeout(500);
  await panel.getByRole('button', { name: 'Erase' }).click();
  await dragRow(page, 80, 70, 90, 'docked');
  await page.waitForTimeout(200);
  const erased = await population(page);

  await panel.getByRole('button', { name: 'Carrier brush' }).click();
  await expect(panel.getByRole('button', { name: 'Carrier brush' })).toHaveAttribute('aria-pressed', 'true');
  await dragRow(page, 80, 72, 81, 'docked');
  await expect.poll(() => population(page)).toBe(erased + 10);
  await expectColourAt(page, centreOf(76, 80, 'docked'), DEFAULT_BRUSH.first.hue);

  const brush = panel.getByRole('region', { name: 'Brush' });
  await brush.getByRole('slider', { name: 'Second dominance' }).focus();
  await page.keyboard.press('End');
  await page.keyboard.press('Control+z');
  await expect.poll(() => population(page)).toBe(erased);
});

test('the brush settings are labelled', async ({ page }) => {
  await openPaused(page);
  await page.getByRole('button', { name: 'Settings' }).click();
  const brush = page.getByRole('dialog', { name: 'Settings' }).getByRole('region', { name: 'Brush' });
  await expect(brush.getByRole('slider', { name: 'Colour', exact: true })).toHaveAttribute('aria-valuetext', `hue ${DEFAULT_BRUSH.first.hue}°`);
  await expect(brush.getByRole('slider', { name: 'Dominance', exact: true })).toHaveAttribute('aria-valuetext', 'strong');
  await expect(brush.getByRole('slider', { name: 'Second colour', exact: true })).toHaveAttribute('aria-valuetext', `hue ${DEFAULT_BRUSH.second.hue}°`);
  await expect(brush.getByRole('slider', { name: 'Second dominance' })).toHaveAttribute('aria-valuetext', 'weakest');
  await brush.getByRole('combobox', { name: 'Pattern' }).selectOption({ label: 'Gosper glider gun' });
  await expect(brush.getByRole('combobox', { name: 'Pattern' })).toHaveValue('gosper-gun');
});
