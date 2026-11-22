import { expect, type Page } from '@playwright/test';
import { stats } from './stats.ts';

/**
 * The opening view: the 160 by 160 World at 9 px per Cell, with the top
 * left of Cell (80, 80) at the middle of the stage. The stage fills the
 * 1440 by 900 viewport, less the 340 px side panel when docked, or the
 * whole 412 by 839 screen of the phone, where the drawer opens over it.
 */
const SCALE = 9;
type Layout = 'floating' | 'docked' | 'phone';
const STAGE_MIDDLE: Record<Layout, { x: number; y: number }> = { floating: { x: 720, y: 450 }, docked: { x: 550, y: 450 }, phone: { x: 206, y: 419.5 } };

export function centreOf(x: number, y: number, layout: Layout = 'floating'): { x: number; y: number } {
  const middle = STAGE_MIDDLE[layout];
  return { x: middle.x + (x - 80) * SCALE + SCALE / 2, y: middle.y + (y - 80) * SCALE + SCALE / 2 };
}

export const population = async (page: Page): Promise<number> => (await stats(page)).population;

export async function openPaused(page: Page): Promise<void> {
  await page.goto('./');
  await expect.poll(async () => (await stats(page)).generation).toBeGreaterThan(2);
  await page.getByRole('button', { name: 'Pause' }).click();
  await page.waitForTimeout(300);
}

/** Drags the current tool along row `y` from column `from` to column `to`. */
export async function dragRow(page: Page, y: number, from: number, to: number, layout: Layout = 'floating'): Promise<void> {
  const start = centreOf(from, y, layout);
  const end = centreOf(to, y, layout);
  await page.mouse.move(start.x, start.y);
  await page.mouse.down();
  await page.mouse.move(end.x, end.y, { steps: 6 });
  await page.mouse.up();
}
