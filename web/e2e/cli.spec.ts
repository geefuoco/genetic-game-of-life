import { execFile } from 'node:child_process';
import { mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';
import { expect, test } from '@playwright/test';
import { exportSnapshot, openPausedAt } from './sharing.ts';

const REPOSITORY = fileURLToPath(new URL('../..', import.meta.url));

/** Runs the CLI with `args`, building it first if needed, and returns the stats it prints. */
async function cli(args: string[]): Promise<{ generation: number }> {
  const { stdout } = await promisify(execFile)('cargo', ['run', '--quiet', '-p', 'genetic-life-cli', '--', ...args], {
    cwd: REPOSITORY,
    maxBuffer: 64 << 20,
  });
  return JSON.parse(stdout) as { generation: number };
}

test('a run in the browser matches the CLI for the same Seed, Rule, settings and number of Generations', async ({ page }) => {
  test.setTimeout(300_000);
  const soup = { seed: '9001', rule: 'B36/S23', width: '96', height: '64', density: '0.35', alleles: '5', 'mutation-rate': '0.05', 'mutation-magnitude': '45' };
  const link = `v=1&${Object.entries(soup)
    .map(([field, value]) => `${field}=${value}`)
    .join('&')}`;
  await openPausedAt(page, `./#${link}`);
  const browser = (await exportSnapshot(page)).bytes;

  const directory = await mkdtemp(join(tmpdir(), 'genetic-life-cli-'));
  const fromBrowser = join(directory, 'browser.ggls');
  const fromCli = join(directory, 'cli.ggls');
  await writeFile(fromBrowser, browser);
  const { generation } = await cli(['--snapshot', fromBrowser, '--generations', '0']);
  expect(generation).toBeGreaterThan(2);

  await cli([
    ...Object.entries(soup).flatMap(([field, value]) => [`--${field}`, value]),
    '--generations',
    String(generation),
    '--out',
    fromCli,
  ]);
  expect((await readFile(fromCli)).equals(browser)).toBe(true);
});
