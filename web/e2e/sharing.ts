import { readFile } from 'node:fs/promises';
import { expect, type Locator, type Page } from '@playwright/test';
import { stats } from './stats.ts';

/** Opens the settings popover. */
export async function openSettings(page: Page): Promise<Locator> {
  const settings = page.getByRole('dialog', { name: 'Settings' });
  if (!(await settings.isVisible())) await page.getByRole('button', { name: 'Settings' }).click();
  await expect(settings).toBeVisible();
  return settings;
}

/** Opens `url`, waits for the World to run, then pauses and waits for any frame in flight. */
export async function openPausedAt(page: Page, url: string): Promise<void> {
  await page.goto(url);
  await expect.poll(async () => (await stats(page)).generation).toBeGreaterThan(2);
  await page.getByRole('button', { name: 'Pause' }).click();
  await page.waitForTimeout(300);
}

/** The address the settings offer for sharing the run. */
export async function shareLink(page: Page): Promise<string> {
  const settings = await openSettings(page);
  return settings.getByRole('textbox', { name: 'Share link' }).inputValue();
}

/** Exports a Snapshot from the settings and returns the downloaded file's name and bytes. */
export async function exportSnapshot(page: Page): Promise<{ name: string; bytes: Buffer }> {
  const settings = await openSettings(page);
  const [download] = await Promise.all([page.waitForEvent('download'), settings.getByRole('button', { name: 'Export snapshot' }).click()]);
  const path = await download.path();
  return { name: download.suggestedFilename(), bytes: await readFile(path) };
}

/** Imports a Snapshot file through the settings. */
export async function importSnapshot(page: Page, bytes: Buffer): Promise<void> {
  const settings = await openSettings(page);
  await settings.getByLabel('Import snapshot').setInputFiles({ name: 'world.ggls', mimeType: 'application/octet-stream', buffer: bytes });
}

/** Steps one Generation at a time until the World reaches `generation`. */
export async function stepTo(page: Page, generation: number): Promise<void> {
  for (let at = (await stats(page)).generation; at < generation; at++) {
    await page.getByRole('button', { name: 'Step one generation' }).click();
    await expect.poll(async () => (await stats(page)).generation).toBe(at + 1);
  }
}
