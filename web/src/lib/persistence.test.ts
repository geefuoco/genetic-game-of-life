import { describe, expect, it } from 'vitest';
import { linkInFragment, openingPlan, rejectionNotice, shareUrl, snapshotFileName } from './persistence.ts';

const LINK = 'v=1&seed=42&rule=B3/S23&width=160&height=160&density=0.3&alleles=8&mutation-rate=0.01&mutation-magnitude=30';
const AUTOSAVED = { snapshot: Uint8Array.of(1, 2, 3), link: null };

describe('the opening plan', () => {
  it('opens a Share Link in the address first, then the autosaved World, then a fresh Soup', () => {
    expect(openingPlan(`#${LINK}`, AUTOSAVED)).toEqual([{ from: 'link', link: LINK }, { from: 'autosave', autosaved: AUTOSAVED }, { from: 'soup' }]);
  });

  it('restores the autosaved World when the address has no Share Link', () => {
    expect(openingPlan('', AUTOSAVED)).toEqual([{ from: 'autosave', autosaved: AUTOSAVED }, { from: 'soup' }]);
    expect(openingPlan('#', AUTOSAVED)).toEqual([{ from: 'autosave', autosaved: AUTOSAVED }, { from: 'soup' }]);
  });

  it('opens a fresh Soup when nothing was autosaved', () => {
    expect(openingPlan('', null)).toEqual([{ from: 'soup' }]);
  });
});

describe('the Share Link in an address fragment', () => {
  it('is the fragment without its #', () => {
    expect(linkInFragment(`#${LINK}`)).toBe(LINK);
  });

  it('is undone of the percent-encoding something along the way may have added', () => {
    expect(linkInFragment('#v=1&seed=42&rule=B3%2FS23')).toBe('v=1&seed=42&rule=B3/S23');
  });

  it('is left as it is when its percent-encoding is malformed, for the core to explain', () => {
    expect(linkInFragment('#v=1&seed=%E0%A4%A')).toBe('v=1&seed=%E0%A4%A');
  });

  it('is empty when the address has no fragment', () => {
    expect(linkInFragment('')).toBe('');
    expect(linkInFragment('#')).toBe('');
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
