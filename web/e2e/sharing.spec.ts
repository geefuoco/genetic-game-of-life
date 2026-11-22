import { expect, test } from '@playwright/test';
import { AUTOSAVE_MS } from '../src/lib/autosave.ts';
import { exportSnapshot, importSnapshot, openPausedAt, openSettings, shareLink, stepTo } from './sharing.ts';
import { stats } from './stats.ts';
import { dragRow } from './world.ts';

test('a share link opens the identical Soup run in another browser', async ({ page, browser }) => {
  await openPausedAt(page, './');
  const url = await shareLink(page);
  expect(new URL(url).hash).toMatch(/^#v=1&seed=\d+&rule=B3\/S23&width=160&height=160&/);

  const other = await (await browser.newContext()).newPage();
  await openPausedAt(other, url);
  expect(new URL(other.url()).hash).toBe('');
  const generation = Math.max((await stats(page)).generation, (await stats(other)).generation);
  await stepTo(page, generation);
  await stepTo(other, generation);
  const [original, opened] = [await exportSnapshot(page), await exportSnapshot(other)];
  expect(opened.bytes.equals(original.bytes)).toBe(true);
  expect(await shareLink(other)).toBe(url);
});

test('an edited World points to Snapshot export instead of a share link', async ({ page }) => {
  await openPausedAt(page, './');
  const settings = await openSettings(page);
  await expect(settings.getByRole('textbox', { name: 'Share link' })).toBeVisible();
  await page.getByRole('button', { name: 'Settings' }).click();
  await page.getByRole('group', { name: 'Tools' }).getByRole('button', { name: 'Paint' }).click();
  await dragRow(page, 80, 78, 82);
  await openSettings(page);
  await expect(settings.getByRole('textbox', { name: 'Share link' })).toHaveCount(0);
  await expect(settings.getByTestId('no-share-link')).toHaveText(/Export a snapshot to keep or share this world/);
  await expect(settings.getByRole('button', { name: 'Export snapshot' })).toBeVisible();
});

test('an exported Snapshot imports to resume the run exactly, saying ancestors restart', async ({ page }) => {
  await openPausedAt(page, './');
  const before = await stats(page);
  const exported = await exportSnapshot(page);
  expect(exported.name).toBe(`genetic-life-gen-${before.generation}.ggls`);

  await stepTo(page, before.generation + 3);
  await importSnapshot(page, exported.bytes);
  await expect(page.getByRole('status').filter({ hasText: 'Ancestors restart from here' })).toBeVisible();
  await expect.poll(() => stats(page)).toEqual(before);
  expect((await exportSnapshot(page)).bytes.equals(exported.bytes)).toBe(true);
});

test('a corrupt Snapshot or share link is refused with a clear message and the World is unchanged', async ({ page }) => {
  await openPausedAt(page, './');
  const before = await stats(page);
  const url = await shareLink(page);

  await importSnapshot(page, Buffer.from('not a snapshot'));
  await expect(page.getByRole('alert')).toHaveText("That snapshot can't be imported: not a snapshot: the data does not start with GGLS. Your world is unchanged.");
  const exported = await exportSnapshot(page);
  await importSnapshot(page, exported.bytes.subarray(0, exported.bytes.length - 20));
  await expect(page.getByRole('alert')).toHaveText(/^That snapshot can't be imported: snapshot data is (truncated|corrupt)/);

  await page.evaluate(() => {
    location.hash = 'v=2&seed=1';
  });
  await expect(page.getByRole('alert')).toHaveText(
    "That share link can't be opened: share link version '2' is not supported, this build reads version 1. Your world is unchanged.",
  );
  await page.waitForTimeout(300);
  expect(await stats(page)).toEqual(before);
  expect(await shareLink(page)).toBe(url);
});

test('a bad share link in the address explains itself and opens a fresh Soup', async ({ page }) => {
  await page.goto('./#v=1&seed=1&rule=B3/S23&width=8&height=8&density=0.3&alleles=4&mutation-rate=0.01&mutation-magnitude=30');
  await expect(page.getByRole('alert')).toHaveText(/^That share link can't be opened: share link world settings are invalid: world size 8x8 is outside/);
  await expect.poll(async () => (await stats(page)).generation).toBeGreaterThan(2);
  expect((await stats(page)).population).toBeGreaterThan(0);
});

test('the World is autosaved and restored on reload, still shareable', async ({ page, browser }) => {
  await page.goto('./');
  const speed = page.getByRole('button', { name: /^Speed/ });
  await speed.click({ modifiers: ['Shift'] });
  await speed.click({ modifiers: ['Shift'] });
  await expect(speed).toHaveText('4×');
  await page.waitForTimeout(2500);
  await page.getByRole('button', { name: 'Pause' }).click();
  await page.waitForTimeout(300);
  const saved = await stats(page);
  expect(saved.generation).toBeGreaterThan(100);
  const url = await shareLink(page);
  const before = await exportSnapshot(page);
  await page.waitForTimeout(AUTOSAVE_MS + 1000);

  await page.reload();
  await expect.poll(async () => (await stats(page)).population).toBeGreaterThan(0);
  await page.getByRole('button', { name: 'Pause' }).click();
  await page.waitForTimeout(300);
  const restored = await stats(page);
  expect(restored.generation).toBeGreaterThanOrEqual(saved.generation);
  expect(await shareLink(page)).toBe(url);

  const other = await (await browser.newContext()).newPage();
  await openPausedAt(other, './');
  await importSnapshot(other, before.bytes);
  await expect.poll(() => stats(other)).toEqual(saved);
  await stepTo(other, restored.generation);
  expect((await exportSnapshot(other)).bytes.equals((await exportSnapshot(page)).bytes)).toBe(true);
});

test('an unreadable autosave falls back to a fresh Soup', async ({ page }) => {
  await page.goto('./');
  await page.evaluate(
    () =>
      new Promise<void>((resolve, reject) => {
        const request = indexedDB.open('genetic-life', 1);
        request.onupgradeneeded = () => request.result.createObjectStore('autosave');
        request.onsuccess = () => {
          const put = request.result.transaction('autosave', 'readwrite').objectStore('autosave').put({ snapshot: Uint8Array.of(1, 2, 3), link: null }, 'world');
          put.onsuccess = () => resolve();
          put.onerror = () => reject(new Error('could not write'));
        };
        request.onerror = () => reject(new Error('could not open'));
      }),
  );
  await page.reload();
  await expect.poll(async () => (await stats(page)).generation).toBeGreaterThan(2);
  expect((await stats(page)).population).toBeGreaterThan(0);
  await expect(page.getByRole('alert')).toHaveCount(0);
});
