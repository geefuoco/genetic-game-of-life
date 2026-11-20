import type { FrameLocator, Page } from '@playwright/test';

/** The Generation and population the page's stats line shows. */
export async function stats(scope: Page | FrameLocator): Promise<{ generation: number; population: number }> {
  const text = (await scope.getByTestId('stats').textContent()) ?? '';
  const match = /gen ([\d,]+) · ([\d,]+) alive/.exec(text);
  if (!match) throw new Error(`unexpected stats line: ${text}`);
  const count = (digits: string | undefined): number => Number((digits ?? '').replaceAll(',', ''));
  return { generation: count(match[1]), population: count(match[2]) };
}
