import { describe, expect, it } from 'vitest';
import { openingPlan, rejectionNotice, shareUrl, snapshotFileName } from './persistence.ts';

const LINK = 'v=1&seed=42&rule=B3/S23&width=160&height=160&density=0.3&alleles=8&mutation-rate=0.01&mutation-magnitude=30';
const SAVED = { snapshot: Uint8Array.of(1, 2, 3), link: null };

describe('the opening plan', () => {
  it('opens a Share Link in the address first, then the autosaved World, then a fresh Soup', () => {
    expect(openingPlan(`#${LINK}`, SAVED)).toEqual([{ from: 'link', link: LINK }, { from: 'autosave', saved: SAVED }, { from: 'soup' }]);
  });

  it('restores the autosaved World when the address has no Share Link', () => {
    expect(openingPlan('', SAVED)).toEqual([{ from: 'autosave', saved: SAVED }, { from: 'soup' }]);
    expect(openingPlan('#', SAVED)).toEqual([{ from: 'autosave', saved: SAVED }, { from: 'soup' }]);
  });

  it('opens a fresh Soup when nothing was autosaved', () => {
    expect(openingPlan('', null)).toEqual([{ from: 'soup' }]);
  });
});

describe('share URLs', () => {
  it('put the Share Link in the fragment of the page address, keeping its path and query', () => {
    expect(shareUrl('https://example.com/genetic-game-of-life/?embed=1', LINK)).toBe(`https://example.com/genetic-game-of-life/?embed=1#${LINK}`);
  });

  it('replace a fragment already in the address', () => {
    expect(shareUrl('https://example.com/app/#v=1&seed=1', LINK)).toBe(`https://example.com/app/#${LINK}`);
  });
});

describe('Snapshot files', () => {
  it('are named after the Generation they hold', () => {
    expect(snapshotFileName(1234)).toBe('genetic-life-gen-1234.ggls');
  });
});

describe('rejection notices', () => {
  it('give the core explanation and say the World is unchanged', () => {
    expect(rejectionNotice('snapshot', 'snapshot data is truncated', true)).toBe(
      "That snapshot can't be imported: snapshot data is truncated. Your world is unchanged.",
    );
    expect(rejectionNotice('link', "share link is missing the 'seed' field", true)).toBe(
      "That share link can't be opened: share link is missing the 'seed' field. Your world is unchanged.",
    );
  });

  it('only give the explanation when no World was running yet', () => {
    expect(rejectionNotice('link', "share link is missing the 'seed' field", false)).toBe(
      "That share link can't be opened: share link is missing the 'seed' field.",
    );
  });
});
