import { describe, expect, it } from 'vitest';
import { densityLabel, mutationRateLabel, mutationMagnitudeLabel, presetOf, readSoupDraft, type SoupDraft } from './settings.ts';

describe('Rule Presets', () => {
  it('names the Rule Preset a canonical notation belongs to', () => {
    expect(presetOf('B36/S23')?.label).toBe('HighLife');
    expect(presetOf('B3678/S34678')?.label).toBe('Day & Night');
  });

  it('names none for a custom Rule', () => {
    expect(presetOf('B3/S2')).toBeNull();
  });
});

describe('value labels', () => {
  it('show the Mutation rate as a percentage to one decimal place', () => {
    expect(mutationRateLabel(0.01)).toBe('1.0%');
    expect(mutationRateLabel(0.005)).toBe('0.5%');
    expect(mutationRateLabel(0)).toBe('0.0%');
  });

  it('show the Mutation magnitude as a reach either way, in degrees', () => {
    expect(mutationMagnitudeLabel(30)).toBe('±30°');
  });

  it('show density as a whole percentage', () => {
    expect(densityLabel(0.3)).toBe('30%');
    expect(densityLabel(0.07)).toBe('7%');
  });
});

describe('reading the New Soup form', () => {
  const DRAFT: SoupDraft = { foundingAlleles: 8, density: 0.3, width: 256, height: 128, seed: '42' };
  const noRandom = (): bigint => {
    throw new Error('a random Seed was drawn');
  };

  it('gives the Soup settings the form describes', () => {
    expect(readSoupDraft(DRAFT, noRandom)).toEqual({ ok: true, soup: { seed: 42n, width: 256, height: 128, density: 0.3, foundingAlleles: 8 } });
  });

  it('draws a random Seed when the Seed is left empty', () => {
    const result = readSoupDraft({ ...DRAFT, seed: '  ' }, () => 7n);
    expect(result.ok && result.soup.seed).toBe(7n);
  });

  it('accepts World sizes from 16 to 1024 on each axis', () => {
    expect(readSoupDraft({ ...DRAFT, width: 16, height: 1024 }, noRandom).ok).toBe(true);
  });

  it('accepts the largest Seed', () => {
    const result = readSoupDraft({ ...DRAFT, seed: '18446744073709551615' }, noRandom);
    expect(result.ok && result.soup.seed).toBe(18446744073709551615n);
  });

  it.each(['-1', '1.5', '12abc', '1e3', '18446744073709551616'])('explains a Seed of %s', (seed) => {
    expect(readSoupDraft({ ...DRAFT, seed }, noRandom)).toEqual({
      ok: false,
      problems: { seed: 'Seed must be a whole number from 0 to 18446744073709551615, or empty for a random one.' },
    });
  });

  it.each([15, 1025, 100.5, null])('explains a World width or height of %s', (size) => {
    expect(readSoupDraft({ ...DRAFT, width: size, height: size }, noRandom)).toEqual({
      ok: false,
      problems: { width: 'Width must be a whole number from 16 to 1024.', height: 'Height must be a whole number from 16 to 1024.' },
    });
  });
});
