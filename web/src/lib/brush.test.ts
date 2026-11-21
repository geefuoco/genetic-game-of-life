import { describe, expect, it } from 'vitest';
import { DEFAULT_BRUSH, brushGenotype, dominanceLabel } from './brush.ts';

describe('the brush', () => {
  it('paints homozygous Organisms by default', () => {
    const brush = { ...DEFAULT_BRUSH, colour: { hue: 200, dominance: 1 } };
    expect(brushGenotype(brush)).toEqual([
      { hue: 200, dominance: 1 },
      { hue: 200, dominance: 1 },
    ]);
  });

  it('paints two different Alleles as the Carrier brush', () => {
    const brush = { ...DEFAULT_BRUSH, colour: { hue: 200, dominance: 3 }, carrier: true, carried: { hue: 30, dominance: 0 } };
    expect(brushGenotype(brush)).toEqual([
      { hue: 200, dominance: 3 },
      { hue: 30, dominance: 0 },
    ]);
  });

  it('opens with a Carrier brush whose second Allele is hidden by the first', () => {
    expect(DEFAULT_BRUSH.carrier).toBe(false);
    expect(DEFAULT_BRUSH.carried.dominance).toBeLessThan(DEFAULT_BRUSH.colour.dominance);
  });

  it('names Dominance Levels from weakest to strongest', () => {
    expect([0, 1, 2, 3].map(dominanceLabel)).toEqual(['weakest', 'weak', 'strong', 'strongest']);
  });
});
