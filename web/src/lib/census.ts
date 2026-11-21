/** What the stats panel shows of a census. */

import type { AlleleCensus } from '../protocol.ts';

/** One living Allele in the stats panel's list. */
export interface AlleleRow {
  hue: number;
  /** Live Cells carrying it, once or twice. */
  carried: number;
  /** Live Cells whose Phenotype shows it. */
  shown: number;
  /** Whether no live Cell shows it. */
  fullyHidden: boolean;
}

export interface AlleleRows {
  /** The most carried Alleles, most carried first. */
  rows: AlleleRow[];
  /** How many living Alleles are left out of `rows`. */
  more: number;
  /** Alleles carried by a live Cell, shown or hidden. */
  livingCount: number;
  /** Living Alleles no live Cell shows. */
  fullyHiddenCount: number;
}

/** The census's Alleles as rows comparing carried with shown, keeping the `limit` most carried. */
export function alleleRows(alleles: readonly AlleleCensus[], limit: number): AlleleRows {
  const all = alleles.map(({ hue, shown, hidden }): AlleleRow => ({ hue, carried: shown + hidden, shown, fullyHidden: shown === 0 }));
  all.sort((a, b) => b.carried - a.carried);
  const rows = all.slice(0, limit);
  return {
    rows,
    more: all.length - rows.length,
    livingCount: all.length,
    fullyHiddenCount: all.filter((row) => row.fullyHidden).length,
  };
}

/** The population at one Generation. */
export interface HistoryPoint {
  generation: number;
  population: number;
}

/** The first Generations with the highest and the lowest population. */
export function historyExtremes(history: ArrayLike<number>): { peak: HistoryPoint; lowest: HistoryPoint } {
  let peak = 0;
  let lowest = 0;
  for (let generation = 1; generation < history.length; generation++) {
    const population = history[generation] ?? 0;
    if (population > (history[peak] ?? 0)) peak = generation;
    if (population < (history[lowest] ?? 0)) lowest = generation;
  }
  return {
    peak: { generation: peak, population: history[peak] ?? 0 },
    lowest: { generation: lowest, population: history[lowest] ?? 0 },
  };
}

/**
 * The population history reduced to at most `2 × buckets` points for drawing: the first and latest Generations, and
 * in between the lowest and highest population of each of `buckets - 1` equal spans of Generations, in Generation
 * order, so a boom or collapse lasting one Generation still shows. `buckets` below 2 counts as 2.
 */
export function reducedHistory(history: ArrayLike<number>, buckets: number): HistoryPoint[] {
  const point = (generation: number): HistoryPoint => ({ generation, population: history[generation] ?? 0 });
  const count = history.length;
  const kept = Math.max(2, buckets);
  if (count <= 2 * kept) return Array.from({ length: count }, (_, generation) => point(generation));
  const points = [point(0)];
  const interior = count - 2;
  const spans = kept - 1;
  for (let span = 0; span < spans; span++) {
    const start = 1 + Math.floor((span * interior) / spans);
    const end = 1 + Math.floor(((span + 1) * interior) / spans);
    if (start >= end) continue;
    let lowest = start;
    let highest = start;
    for (let generation = start + 1; generation < end; generation++) {
      const population = history[generation] ?? 0;
      if (population < (history[lowest] ?? 0)) lowest = generation;
      if (population > (history[highest] ?? 0)) highest = generation;
    }
    const first = Math.min(lowest, highest);
    const second = Math.max(lowest, highest);
    points.push(point(first));
    if (second !== first) points.push(point(second));
  }
  points.push(point(count - 1));
  return points;
}
