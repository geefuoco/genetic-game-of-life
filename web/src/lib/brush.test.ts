import { describe, expect, it } from 'vitest';
import { DEFAULT_BRUSH, dominanceLabel, toolGenotype } from './brush.ts';

const brush = { ...DEFAULT_BRUSH, first: { hue: 200, dominance: 3 }, second: { hue: 30, dominance: 0 } };

describe('the brush', () => {
  it('paints and stamps live Cells homozygous for its first Allele', () => {
    const homozygous = [
      { hue: 200, dominance: 3 },
      { hue: 200, dominance: 3 },
    ];
    expect(toolGenotype('paint', brush)).toEqual(homozygous);
    expect(toolGenotype('stamp', brush)).toEqual(homozygous);
  });

  it('paints Carriers of both its Alleles with the Carrier brush', () => {
    expect(toolGenotype('carrier', brush)).toEqual([
      { hue: 200, dominance: 3 },
      { hue: 30, dominance: 0 },
    ]);
  });

  it('opens with a second Allele that the first hides', () => {
    expect(DEFAULT_BRUSH.second.dominance).toBeLessThan(DEFAULT_BRUSH.first.dominance);
    expect(DEFAULT_BRUSH.second.hue).not.toBe(DEFAULT_BRUSH.first.hue);
  });

  it('names Dominance Levels from weakest to strongest', () => {
    expect([0, 1, 2, 3].map(dominanceLabel)).toEqual(['weakest', 'weak', 'strong', 'strongest']);
  });
});
