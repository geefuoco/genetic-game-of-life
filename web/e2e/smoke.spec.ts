import { expect, test, type FrameLocator, type Page } from '@playwright/test';

async function stats(scope: Page | FrameLocator): Promise<{ generation: number; population: number }> {
  const text = (await scope.getByTestId('stats').textContent()) ?? '';
  const match = /gen ([\d,]+) · ([\d,]+) alive/.exec(text);
  if (!match) throw new Error(`unexpected stats line: ${text}`);
  const number = (digits: string | undefined): number => Number((digits ?? '').replaceAll(',', ''));
  return { generation: number(match[1]), population: number(match[2]) };
}

const generation = async (scope: Page | FrameLocator): Promise<number> => (await stats(scope)).generation;

/** Waits until the World has advanced, so the page is running. */
async function open(page: Page): Promise<void> {
  await page.goto('./');
  await expect.poll(() => generation(page)).toBeGreaterThan(2);
}

/** Pauses and waits for any frame still in flight. */
async function pause(page: Page): Promise<number> {
  await page.getByRole('button', { name: 'Pause' }).click();
  await page.waitForTimeout(300);
  return generation(page);
}

/** Generations the World advances over `ms` of running. */
async function advanceOver(page: Page, ms: number): Promise<number> {
  const before = await generation(page);
  await page.waitForTimeout(ms);
  return (await generation(page)) - before;
}

async function nextPaint(page: Page): Promise<void> {
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
}

/** RGBA pixels of a page screenshot, decoded in the browser. */
async function screenshotPixels(page: Page): Promise<{ width: number; height: number; data: number[] }> {
  const png = (await page.screenshot()).toString('base64');
  return page.evaluate(async (base64) => {
    const bytes = Uint8Array.from(atob(base64), (c) => c.charCodeAt(0));
    const bitmap = await createImageBitmap(new Blob([bytes], { type: 'image/png' }));
    const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
    const context = canvas.getContext('2d');
    if (!context) throw new Error('no 2D context');
    context.drawImage(bitmap, 0, 0);
    return { width: bitmap.width, height: bitmap.height, data: Array.from(context.getImageData(0, 0, bitmap.width, bitmap.height).data) };
  }, png);
}

test('opens on a running Soup with live Cells', async ({ page }) => {
  await open(page);
  await expect(page.getByRole('heading', { name: 'genetic life' })).toBeVisible();
  expect((await stats(page)).population).toBeGreaterThan(0);
  await expect(page.getByText('drag to move · scroll to zoom')).toBeVisible();
});

test('makes no requests to other origins and uses the bundled Inter', async ({ page, baseURL }) => {
  const origin = new URL(baseURL ?? '').origin;
  const external: string[] = [];
  page.on('request', (request) => {
    const url = new URL(request.url());
    if (url.protocol.startsWith('http') && url.origin !== origin) external.push(request.url());
  });
  await open(page);
  await page.waitForLoadState('networkidle');
  expect(external).toEqual([]);
  const interLoaded = await page.evaluate(async () => {
    await document.fonts.ready;
    return document.fonts.check('500 15px "Inter Variable"');
  });
  expect(interLoaded).toBe(true);
});

test('runs without cross-origin isolation', async ({ page }) => {
  await open(page);
  expect(await page.evaluate(() => self.crossOriginIsolated)).toBe(false);
});

test('runs embedded in an iframe of a page on another origin', async ({ page, baseURL }) => {
  await page.setContent(`<iframe src="${baseURL}" width="1200" height="800"></iframe>`);
  const frame = page.frameLocator('iframe');
  await expect.poll(() => generation(frame)).toBeGreaterThan(2);
  expect((await stats(frame)).population).toBeGreaterThan(0);
});

test('pauses, steps one Generation at a time, and plays again', async ({ page }) => {
  await open(page);
  const paused = await pause(page);
  expect(await advanceOver(page, 500)).toBe(0);
  await page.getByRole('button', { name: 'Step one generation' }).click();
  await expect.poll(() => generation(page)).toBe(paused + 1);
  await page.getByRole('button', { name: 'Play' }).click();
  await expect.poll(() => generation(page)).toBeGreaterThan(paused + 3);
});

test('stepping while running pauses', async ({ page }) => {
  await open(page);
  await page.getByRole('button', { name: 'Step one generation' }).click();
  await expect(page.getByRole('button', { name: 'Play' })).toBeVisible();
  await page.waitForTimeout(300);
  expect(await advanceOver(page, 500)).toBe(0);
});

test('cycles speed faster to Max and round, and slower with Shift+click or right-click', async ({ page }) => {
  await open(page);
  const speed = page.getByRole('button', { name: /^Speed/ });
  await expect(speed).toHaveText('1×');
  for (const label of ['2×', '4×', 'Max', '½×']) {
    await speed.click();
    await expect(speed).toHaveText(label);
  }
  await speed.click({ modifiers: ['Shift'] });
  await expect(speed).toHaveText('Max');
  await speed.click({ button: 'right' });
  await expect(speed).toHaveText('4×');
});

test('runs far faster at Max than at 1×', async ({ page }) => {
  await open(page);
  const normal = await advanceOver(page, 1000);
  const speed = page.getByRole('button', { name: /^Speed/ });
  await speed.click({ modifiers: ['Shift'] });
  await speed.click({ modifiers: ['Shift'] });
  await expect(speed).toHaveText('Max');
  const max = await advanceOver(page, 1000);
  expect(normal).toBeGreaterThan(8);
  expect(normal).toBeLessThan(25);
  expect(max).toBeGreaterThan(normal * 3);
});

test('docks to the side and floats again, remembering the choice across reloads', async ({ page }) => {
  await open(page);
  await page.getByRole('button', { name: 'Dock the controls to the side' }).click();
  const panel = page.getByRole('complementary', { name: 'Side panel' });
  await expect(panel.getByRole('button', { name: 'Float the controls' })).toBeVisible();
  await expect(panel.getByRole('button', { name: 'Float the controls' })).toBeFocused();
  await page.reload();
  await expect(panel.getByRole('button', { name: 'Float the controls' })).toBeVisible();
  await panel.getByRole('button', { name: 'Float the controls' }).click();
  await expect(page.getByRole('button', { name: 'Dock the controls to the side' })).toBeVisible();
  await expect(panel).toBeHidden();
  await page.reload();
  await expect(page.getByRole('button', { name: 'Dock the controls to the side' })).toBeVisible();
});

test('keyboard shortcuts play, pause, step, change speed and dock', async ({ page }) => {
  await open(page);
  await page.keyboard.press('Space');
  await expect(page.getByRole('button', { name: 'Play' })).toBeVisible();
  await page.waitForTimeout(300);
  const paused = await generation(page);
  await page.keyboard.press('.');
  await expect.poll(() => generation(page)).toBe(paused + 1);
  const speed = page.getByRole('button', { name: /^Speed/ });
  await page.keyboard.press(']');
  await expect(speed).toHaveText('2×');
  await page.keyboard.press('[');
  await page.keyboard.press('[');
  await page.keyboard.press('[');
  await expect(speed).toHaveText('½×');
  await page.keyboard.press('d');
  await expect(page.getByRole('button', { name: 'Float the controls' })).toBeVisible();
  await page.keyboard.press('Space');
  await expect(page.getByRole('button', { name: 'Pause' })).toBeVisible();
});

test('Space plays or pauses even while a dock button has focus', async ({ page }) => {
  await open(page);
  await page.getByRole('button', { name: 'Step one generation' }).focus();
  await page.keyboard.press('Space');
  await expect(page.getByRole('button', { name: 'Play' })).toBeVisible();
  await page.waitForTimeout(300);
  const paused = await generation(page);
  await page.keyboard.press('Space');
  await expect(page.getByRole('button', { name: 'Pause' })).toBeVisible();
  await expect.poll(() => generation(page)).toBeGreaterThan(paused + 1);
});

test('every control is reachable with Tab and labelled', async ({ page }) => {
  await open(page);
  const reached: string[] = [];
  for (let i = 0; i < 7; i++) {
    await page.keyboard.press('Tab');
    reached.push(await page.evaluate(() => document.activeElement?.getAttribute('aria-label') ?? ''));
  }
  expect(reached).toEqual(['Pause', 'Step one generation', 'Speed 1×', 'Zoom out', 'Zoom in', 'Settings', 'Dock the controls to the side']);
});

test('panning by a whole World width wraps round to the same view', async ({ page }) => {
  await open(page);
  await pause(page);
  const region = { x: 0, y: 120, width: 1440, height: 620 };
  const before = await page.screenshot({ clip: region });
  for (let i = 0; i < 2; i++) {
    await page.mouse.move(300, 450);
    await page.mouse.down();
    await page.mouse.move(1020, 450, { steps: 8 });
    await page.mouse.up();
  }
  await nextPaint(page);
  const after = await page.screenshot({ clip: region });
  expect(after.equals(before)).toBe(true);
  await page.mouse.move(300, 450);
  await page.mouse.down();
  await page.mouse.move(345, 450, { steps: 4 });
  await page.mouse.up();
  await nextPaint(page);
  expect((await page.screenshot({ clip: region })).equals(before)).toBe(false);
});

test('zoomed out, one copy of the World shows inside dark surroundings', async ({ page }) => {
  await open(page);
  await pause(page);
  const zoomOut = page.getByRole('button', { name: 'Zoom out' });
  for (let i = 0; i < 12; i++) await zoomOut.click();
  await page.mouse.move(700, 450);
  await page.mouse.down();
  await page.mouse.move(737, 431, { steps: 4 });
  await page.mouse.up();
  await nextPaint(page);
  const { width, data } = await screenshotPixels(page);
  const brightest = (x0: number, x1: number, y0: number, y1: number): number => {
    let max = 0;
    for (let y = y0; y < y1; y++) {
      for (let x = x0; x < x1; x++) {
        const i = (y * width + x) * 4;
        max = Math.max(max, data[i] ?? 0, data[i + 1] ?? 0, data[i + 2] ?? 0);
      }
    }
    return max;
  };
  expect(brightest(0, 300, 100, 800)).toBeLessThanOrEqual(12);
  expect(brightest(1140, 1440, 100, 800)).toBeLessThanOrEqual(12);
  expect(brightest(500, 940, 250, 650)).toBeGreaterThan(100);
});

test('explains when WebGL2 is unavailable', async ({ page }) => {
  await page.addInitScript(() => {
    const original = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function (this: HTMLCanvasElement, type: string, options?: unknown) {
      return type === 'webgl2' ? null : original.call(this, type, options);
    } as typeof original;
  });
  await page.goto('./');
  await expect(page.getByRole('alert')).toHaveText(/needs WebGL2/);
  await expect(page.getByRole('button', { name: 'Pause' })).toBeHidden();
});
