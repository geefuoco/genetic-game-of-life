import { describe, expect, it } from 'vitest';
import type { AlleleCensus } from '../protocol.ts';
import { alleleRows, historyExtremes, reducedHistory } from './census.ts';

function line(hue: number, shown: number, hidden: number, copies = shown + hidden): AlleleCensus {
  return { id: BigInt(hue), hue, dominance: 1, copies, shown, hidden };
}

describe('Allele rows', () => {
  it('compare how many live Cells carry each Allele with how many show it', () => {
    const { rows } = alleleRows([line(40, 30, 10), line(160, 0, 12)], 10);
    expect(rows).toEqual([
      { hue: 40, carried: 40, shown: 30, fullyHidden: false },
      { hue: 160, carried: 12, shown: 0, fullyHidden: true },
    ]);
  });

  it('count a live Cell carrying an Allele twice as one carrier', () => {
    const { rows } = alleleRows([line(40, 5, 0, 10)], 10);
    expect(rows[0]?.carried).toBe(5);
  });

  it('list the most carried first and leave the rest as a count', () => {
    const { rows, more } = alleleRows([line(10, 1, 0), line(20, 9, 0), line(30, 0, 4), line(40, 2, 2)], 3);
    expect(rows.map((row) => row.hue)).toEqual([20, 30, 40]);
    expect(more).toBe(1);
  });

  it('count the living and the fully hidden Alleles, including those left out of the list', () => {
    const { livingCount, fullyHiddenCount } = alleleRows([line(10, 0, 1), line(20, 9, 0), line(30, 0, 4)], 1);
    expect(livingCount).toBe(3);
    expect(fullyHiddenCount).toBe(2);
  });
});

describe('the reduced population history', () => {
  it('keeps every Generation of a short history', () => {
    expect(reducedHistory([5, 7, 6], 10)).toEqual([
      { generation: 0, population: 5 },
      { generation: 1, population: 7 },
      { generation: 2, population: 6 },
    ]);
  });

  it('keeps a brief boom and a brief collapse in a long history', () => {
    const history = Array.from({ length: 10_000 }, () => 500);
    history[3_141] = 2_000;
    history[7_000] = 3;
    const points = reducedHistory(history, 100);
    expect(points.length).toBeLessThanOrEqual(200);
    expect(points).toContainEqual({ generation: 3_141, population: 2_000 });
    expect(points).toContainEqual({ generation: 7_000, population: 3 });
  });

  it('starts at Generation 0 and ends at the latest Generation, in order', () => {
    const history = Array.from({ length: 1_001 }, (_, generation) => generation % 37);
    const points = reducedHistory(history, 64);
    expect(points[0]?.generation).toBe(0);
    expect(points.at(-1)).toEqual({ generation: 1_000, population: 1_000 % 37 });
    const generations = points.map((point) => point.generation);
    expect(generations).toEqual([...generations].sort((a, b) => a - b));
    expect(new Set(generations).size).toBe(generations.length);
  });
});

describe('the population history extremes', () => {
  it('are the first Generations with the highest and lowest population', () => {
    expect(historyExtremes([40, 90, 12, 90, 12, 50])).toEqual({
      peak: { generation: 1, population: 90 },
      lowest: { generation: 2, population: 12 },
    });
  });
});
