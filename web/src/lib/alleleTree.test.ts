import { describe, expect, it } from 'vitest';
import type { AlleleCensus, AlleleNode, Census } from '../protocol.ts';
import { lineage, byHue, describeLine, lineAt, alleleLines, type AlleleLine } from './alleleTree.ts';

const GENERATION = 100;

function node(id: number, hue: number, source: number | null, origin: number, status: AlleleNode['status'] = 'living', lastCarried = GENERATION): AlleleNode {
  return { id: BigInt(id), hue, dominance: 1, source: source === null ? null : BigInt(source), origin, lastCarried, status };
}

function line(id: number, hue: number, copies: number, shown = copies): AlleleCensus {
  return { id: BigInt(id), hue, dominance: 1, copies, shown, hidden: copies - shown };
}

function census(tree: AlleleNode[], alleles: AlleleCensus[], population: number): Census {
  return { type: 'census', generation: GENERATION, population, alleles, tree, populationHistory: new Uint32Array(GENERATION + 1) };
}

const ids = (found: readonly AlleleLine[]): number[] => found.map((line) => Number(line.id));

describe('alleleLines', () => {
  it('are major for the 16 most carried living Alleles and their ancestors, and minor for the rest', () => {
    const tree = [node(0, 10, null, 0, 'extinct', 40), ...Array.from({ length: 17 }, (_, i) => node(i + 1, 20 + i, 0, 30))];
    const alleles = Array.from({ length: 17 }, (_, i) => line(i + 1, 20 + i, i === 16 ? 10 : 100));
    const drawn = alleleLines(census(tree, alleles, 1000));
    expect(ids(drawn.filter((line) => line.major))).toEqual(Array.from({ length: 17 }, (_, i) => i));
    expect(drawn.find((line) => line.id === 17n)?.major).toBe(false);
  });

  it('are major beyond the 16 most carried when they carry at least 1% of the copies', () => {
    const tree = Array.from({ length: 18 }, (_, i) => node(i, 10 * i, null, 0));
    const alleles = Array.from({ length: 18 }, (_, i) => line(i, 10 * i, 20));
    expect(alleleLines(census(tree, alleles, 1000)).every((line) => line.major)).toBe(true);
  });

  it('give each living Allele its share of the copies, whether any Cell shows it, and the hue it mutated from', () => {
    const tree = [node(0, 10, null, 0), node(1, 50, 0, 20)];
    const drawn = alleleLines(census(tree, [line(0, 10, 300), line(1, 50, 100, 0)], 200));
    expect(drawn).toMatchObject([
      { id: 0n, share: 0.75, shown: 300, sourceHue: null },
      { id: 1n, share: 0.25, shown: 0, sourceHue: 10 },
    ]);
  });
});

describe('lineage', () => {
  it('runs from an Allele back to its Founding Allele', () => {
    const tree = [node(0, 10, null, 0), node(1, 20, 0, 5), node(2, 30, 1, 9), node(3, 40, 0, 9)];
    const drawn = alleleLines(census(tree, [], 10));
    expect([...lineage(drawn, 2n)]).toEqual([2n, 1n, 0n]);
  });
});

describe('finding a line', () => {
  /** 100 px of Generations and 360 px of hue inside the chart's insets, so x = 16 + Generation and y = 6 + hue. */
  const layout = { width: 130, height: 372, generation: GENERATION };
  const drawn = alleleLines(census([node(1, 100, null, 10, 'extinct', 50), node(2, 104, null, 0)], [line(2, 104, 10)], 10));

  it('finds the line nearest the pointer', () => {
    expect(lineAt(drawn, layout, { x: 40, y: 105 })?.id).toBe(1n);
    expect(lineAt(drawn, layout, { x: 40, y: 109 })?.id).toBe(2n);
  });

  it('passes over a line that ended before the pointer', () => {
    expect(lineAt(drawn, layout, { x: 90, y: 105 })?.id).toBe(2n);
  });

  it('finds nothing beyond reach of every line', () => {
    expect(lineAt(drawn, layout, { x: 40, y: 130 })).toBeNull();
  });
});

describe('keyboard order', () => {
  it('runs down the chart by hue, older Alleles first at the same hue', () => {
    const tree = [node(0, 200, null, 0), node(1, 20, null, 0), node(2, 200, 0, 10), node(3, 90, 1, 3)];
    expect(ids(byHue(alleleLines(census(tree, [], 10))))).toEqual([1, 3, 0, 2]);
  });
});

describe('describing a line', () => {
  const tree = [
    node(0, 40, null, 0, 'extinct', 61),
    node(1, 160, 0, 12),
    node(2, 260, null, 30),
    node(3, 262, 2, 40, 'hidden'),
  ];
  const drawn = alleleLines(census(tree, [line(1, 160, 50, 30), line(2, 260, 20), line(3, 262, 30, 0)], 50));
  const described = (id: bigint): ReturnType<typeof describeLine> => {
    const line = drawn.find((found) => found.id === id);
    if (!line) throw new Error(`no line ${id}`);
    return describeLine(line);
  };

  it('tells a founding colour gene that died out', () => {
    expect(described(0n)).toEqual({ name: 'orange', origin: 'A founding colour gene', now: 'Died out after gen 61' });
  });

  it('tells a mutant that is shown', () => {
    expect(described(1n)).toEqual({ name: 'jade', origin: 'Mutated from orange at gen 12', now: '50% of colour genes, shown by 30 cells' });
  });

  it('tells a colour gene painted or stamped in, and one hidden everywhere', () => {
    expect(described(2n).origin).toBe('Added at gen 30');
    expect(described(3n).now).toBe('30% of colour genes, hidden everywhere');
  });
});
