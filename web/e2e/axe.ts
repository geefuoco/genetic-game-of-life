import AxeBuilder from '@axe-core/playwright';
import type { Page } from '@playwright/test';

/** The rules axe finds broken on the page, each with the elements that break it. */
export async function accessibilityViolations(page: Page): Promise<string[]> {
  const results = await new AxeBuilder({ page }).analyze();
  return results.violations.map((violation) => `${violation.id}: ${violation.nodes.map((node) => node.target.join(' ')).join(', ')}`);
}
