import AxeBuilder from '@axe-core/playwright';
import { expect, test, type Page } from '@playwright/test';
import { stats } from './stats.ts';

/**
 * The opening view on a 412 by 839 phone: the 160 by 160 World at 9 px per
 * Cell, with the top left of Cell (80, 80) at the middle of the screen. The
 * drawer opens over the World, so the stage always fills the screen.
 */
const SCALE = 9;
const MIDDLE = { x: 206, y: 419.5 };

interface Point {
  x: number;
  y: number;
}

function centreOf(x: number, y: number, scale = SCALE): Point {
  return { x: MIDDLE.x + (x - 80) * scale + scale / 2, y: MIDDLE.y + (y - 80) * scale + scale / 2 };
}

/** How long fingers rest before lifting after moving, so the browser sees a drag that stopped rather than a fling, which would swallow the next tap. */
const REST_MS = 150;

/** Moves one finger per entry from `from` to `to` together, then rests and lifts them all. */
async function touch(page: Page, fingers: { from: Point; to: Point }[], steps = 8): Promise<void> {
  const cdp = await page.context().newCDPSession(page);
  const at = (t: number): { x: number; y: number; id: number }[] =>
    fingers.map(({ from, to }, id) => ({ x: from.x + (to.x - from.x) * t, y: from.y + (to.y - from.y) * t, id }));
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: at(0) });
  for (let i = 1; i <= steps; i++) await cdp.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: at(i / steps) });
  await page.waitForTimeout(REST_MS);
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  await cdp.detach();
}

async function open(page: Page): Promise<void> {
  await page.goto('./');
  await expect.poll(async () => (await stats(page)).generation).toBeGreaterThan(2);
}

async function openPaused(page: Page): Promise<void> {
  await open(page);
  await page.getByRole('button', { name: 'Pause' }).tap();
  await page.waitForTimeout(300);
}

/** Opens the drawer, picks a tool and closes the drawer again. */
async function pickTool(page: Page, name: string): Promise<void> {
  await page.getByRole('button', { name: 'Open the controls' }).tap();
  const drawer = page.getByRole('complementary', { name: 'Controls drawer' });
  await drawer.getByRole('group', { name: 'Tools' }).getByRole('button', { name }).tap();
  await drawer.getByRole('button', { name: 'Close the controls' }).tap();
  await expect(drawer).toBeHidden();
}

/** Clears rows 76 to 84 around the middle with the eraser, then paints one lone Carrier at (80, 80) and goes back to looking. */
async function paintLoneCarrier(page: Page): Promise<void> {
  await pickTool(page, 'Erase');
  for (let y = 76; y <= 84; y++) await touch(page, [{ from: centreOf(70, y), to: centreOf(90, y) }]);
  await pickTool(page, 'Carrier brush');
  const cell = centreOf(80, 80);
  await page.touchscreen.tap(cell.x, cell.y);
  await pickTool(page, 'Look');
}

async function violations(page: Page): Promise<string[]> {
  const results = await new AxeBuilder({ page }).analyze();
  return results.violations.map((violation) => `${violation.id}: ${violation.nodes.map((node) => node.target.join(' ')).join(', ')}`);
}

test('opens on a phone with the run controls in a small bar and touch hints', async ({ page }) => {
  await open(page);
  await expect(page.getByRole('heading', { name: 'genetic life' })).toBeVisible();
  await expect(page.getByText('tap a cell to see its colour genes · drag to move · pinch to zoom')).toBeVisible();
  const controls = page.getByRole('group', { name: 'Simulation controls' });
  await expect(controls.getByRole('button', { name: 'Pause' })).toBeVisible();
  await expect(controls.getByRole('button', { name: 'Step one generation' })).toBeVisible();
  await expect(controls.getByRole('button', { name: /^Speed/ })).toBeVisible();
  await expect(controls.getByRole('button', { name: 'Open the controls' })).toBeVisible();
  await expect(page.getByRole('group', { name: 'Tools' })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Dock the controls to the side' })).toHaveCount(0);

  await controls.getByRole('button', { name: 'Pause' }).tap();
  await expect(controls.getByRole('button', { name: 'Play' })).toBeVisible();
  await page.waitForTimeout(300);
  const paused = (await stats(page)).generation;
  await controls.getByRole('button', { name: 'Step one generation' }).tap();
  await expect.poll(async () => (await stats(page)).generation).toBe(paused + 1);
});

test('the controls drawer holds every control, and closes with its button, Escape or D', async ({ page }) => {
  await open(page);
  const opener = page.getByRole('button', { name: 'Open the controls' });
  const drawer = page.getByRole('complementary', { name: 'Controls drawer' });
  await expect(drawer).toBeHidden();

  await opener.tap();
  await expect(drawer).toBeVisible();
  const close = drawer.getByRole('button', { name: 'Close the controls' });
  await expect(close).toBeFocused();
  for (const name of ['Pause', 'Step one generation', 'Look', 'Paint', 'Carrier brush', 'Erase', 'Stamp a pattern', 'Undo', 'Redo', 'Zoom out', 'Zoom in']) {
    await expect(drawer.getByRole('button', { name, exact: true })).toBeVisible();
  }
  await expect(drawer.getByRole('heading', { name: 'Settings' })).toBeVisible();
  await expect(drawer).toBeInViewport({ ratio: 1 });

  await close.tap();
  await expect(drawer).toBeHidden();
  await expect(opener).toBeFocused();

  await opener.tap();
  await page.keyboard.press('Escape');
  await expect(drawer).toBeHidden();
  await expect(opener).toBeFocused();

  await page.keyboard.press('d');
  await expect(drawer).toBeVisible();
  await page.keyboard.press('d');
  await expect(drawer).toBeHidden();

  await page.reload();
  await expect(opener).toBeVisible();
  await expect(drawer).toBeHidden();
});

test('tapping an Organism inspects it', async ({ page }) => {
  await openPaused(page);
  await paintLoneCarrier(page);
  const cell = centreOf(80, 80);
  await page.touchscreen.tap(cell.x, cell.y);
  const inspector = page.getByRole('region', { name: 'Red cell' });
  await expect(inspector).toBeVisible();
  await expect(inspector).toBeInViewport({ ratio: 1 });
});

/** At twice the zoom, the middle of Cell (80, 80) is over Cell (81, 81) at the opening zoom, which the eraser cleared. */
test('pinching zooms about the point between the fingers without inspecting', async ({ page }) => {
  await openPaused(page);
  await paintLoneCarrier(page);
  const beside = centreOf(80, 80, SCALE * 2);

  await touch(page, [
    { from: { x: MIDDLE.x - 40, y: MIDDLE.y }, to: { x: MIDDLE.x - 80, y: MIDDLE.y } },
    { from: { x: MIDDLE.x + 40, y: MIDDLE.y }, to: { x: MIDDLE.x + 80, y: MIDDLE.y } },
  ]);
  await page.waitForTimeout(200);
  await expect(page.getByRole('region', { name: /cell$/ })).toHaveCount(0);

  await page.touchscreen.tap(beside.x, beside.y);
  await expect(page.getByRole('region', { name: 'Red cell' })).toBeVisible();
});

test('dragging pans the World and wraps round the torus', async ({ page }) => {
  await openPaused(page);
  await paintLoneCarrier(page);
  const step = (160 * SCALE + 2 * SCALE) / 4;
  for (let i = 0; i < 4; i++) await touch(page, [{ from: { x: 24, y: 600 }, to: { x: 24 + step, y: 600 } }]);
  await page.waitForTimeout(200);
  await expect(page.getByRole('region', { name: /cell$/ })).toHaveCount(0);

  const moved = centreOf(82, 80);
  await page.touchscreen.tap(moved.x, moved.y);
  await expect(page.getByRole('region', { name: 'Red cell' })).toBeVisible();
});

test('the phone view passes an automated accessibility check, with the drawer closed and open', async ({ page }) => {
  await open(page);
  expect(await violations(page)).toEqual([]);
  await page.getByRole('button', { name: 'Open the controls' }).tap();
  await expect(page.getByRole('complementary', { name: 'Controls drawer' })).toBeVisible();
  expect(await violations(page)).toEqual([]);
});
