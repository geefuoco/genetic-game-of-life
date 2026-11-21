/** What the stats panel shows of a census. */

import type { AlleleCensus } from '../protocol.ts';

/** One living Allele in the stats panel's list. */
export interface AlleleRow {
  hue: number;
  dominance: number;
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
  living: number;
  /** Living Alleles no live Cell shows. */
  fullyHidden: number;
}

/** The census's Alleles as rows comparing carried with shown, keeping the `limit` most carried. */
export function alleleRows(alleles: readonly AlleleCensus[], limit: number): AlleleRows {
  const all = alleles.map(
    ({ hue, dominance, shown, hidden }): AlleleRow => ({ hue, dominance, carried: shown + hidden, shown, fullyHidden: shown === 0 }),
  );
  all.sort((a, b) => b.carried - a.carried);
  const rows = all.slice(0, limit);
  return { rows, more: all.length - rows.length, living: all.length, fullyHidden: all.filter((row) => row.fullyHidden).length };
}

/** The population at one Generation. */
export interface HistoryPoint {
  generation: number;
  population: number;
}

/**
 * The population history reduced to at most `2 × buckets` points for drawing: the first and latest Generations, and
 * in between the lowest and highest population of each of `buckets - 1` equal spans of Generations, in Generation
 * order, so a boom or collapse lasting one Generation still shows.
 */
export function reducedHistory(history: ArrayLike<number>, buckets: number): HistoryPoint[] {
  const point = (generation: number): HistoryPoint => ({ generation, population: history[generation] ?? 0 });
  const count = history.length;
  if (count <= 2 * buckets) return Array.from({ length: count }, (_, generation) => point(generation));
  const points = [point(0)];
  const interior = count - 2;
  const spans = Math.max(1, buckets - 1);
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
