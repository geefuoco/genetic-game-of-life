/** What the Allele Tree view draws of a census: each Allele's hue over time, from its origin to when it was last carried. */

import type { AlleleId, AlleleStatus, Census } from '../protocol.ts';
import { colourName } from './colours.ts';

/** Living Alleles whose lines are major by rank, most carried first. */
const MAJOR_RANK = 16;
/** Share of the Allele copies from which a living Allele is a major line whatever its rank. */
const MAJOR_SHARE = 0.01;

/** One Allele as the Allele Tree view draws it. */
export interface AlleleLine {
  id: AlleleId;
  hue: number;
  /** The Allele it mutated from, or `null` for a Founding Allele. */
  source: AlleleId | null;
  /** Hue of the Allele it mutated from, or `null` for a Founding Allele. */
  sourceHue: number | null;
  /** The Generation it first appeared in. */
  origin: number;
  /** The latest Generation a live Cell carried it in. */
  lastCarried: number;
  status: AlleleStatus;
  /** Its share, from 0 to 1, of the Allele copies live Cells carry. */
  share: number;
  /** Live Cells whose Phenotype shows it. */
  shown: number;
  /** Whether its line is emphasised: it is among the most carried living Alleles, or carries a large share, or is an ancestor of one that does. */
  major: boolean;
}

/** Every Allele in the census's Allele Tree, each before its descendants. */
export function alleleLines(census: Census): AlleleLine[] {
  const copies = Math.max(1, 2 * census.population);
  const counts = new Map(census.alleles.map((line) => [line.id, line]));
  const hues = new Map(census.tree.map((node) => [node.id, node.hue]));
  const drawn = census.tree.map((node): AlleleLine => {
    const count = counts.get(node.id);
    return {
      id: node.id,
      hue: node.hue,
      source: node.source,
      sourceHue: node.source === null ? null : (hues.get(node.source) ?? null),
      origin: node.origin,
      lastCarried: node.lastCarried,
      status: node.status,
      share: (count?.copies ?? 0) / copies,
      shown: count?.shown ?? 0,
      major: false,
    };
  });
  const byId = new Map(drawn.map((line) => [line.id, line]));
  const ranked = [...census.alleles].sort((a, b) => b.copies - a.copies);
  ranked.forEach((count, rank) => {
    if (rank >= MAJOR_RANK && count.copies / copies < MAJOR_SHARE) return;
    for (const line of descent(byId, count.id)) {
      if (line.major) break;
      line.major = true;
    }
  });
  return drawn;
}

/** The lineage of Allele `id`: its id and the id of each Allele it descends from, back to its Founding Allele; empty if `id` is not drawn. */
export function lineage(drawn: readonly AlleleLine[], id: AlleleId): Set<AlleleId> {
  const byId = new Map(drawn.map((line) => [line.id, line]));
  return new Set(Array.from(descent(byId, id), (line) => line.id));
}

/** The line of Allele `id`, then the line of each Allele it descends from, back to its Founding Allele. */
function* descent(byId: ReadonlyMap<AlleleId, AlleleLine>, id: AlleleId): Generator<AlleleLine> {
  for (let line = byId.get(id); line; line = line.source === null ? undefined : byId.get(line.source)) yield line;
}

/** Space around the plot, in CSS pixels: the hue strip is on the left, and living Alleles end in a dot on the right. */
export const CHART_INSET = { top: 6, right: 14, bottom: 6, left: 16 } as const;

/** The chart's size in CSS pixels and the latest Generation, at its right edge. */
export interface ChartLayout {
  width: number;
  height: number;
  generation: number;
}

/** The x of a Generation. */
export function chartX(layout: ChartLayout, generation: number): number {
  const plot = Math.max(1, layout.width - CHART_INSET.left - CHART_INSET.right);
  return CHART_INSET.left + (generation / Math.max(1, layout.generation)) * plot;
}

/** The y of a hue, with 0° at the top. */
export function chartY(layout: ChartLayout, hue: number): number {
  const plot = Math.max(1, layout.height - CHART_INSET.top - CHART_INSET.bottom);
  return CHART_INSET.top + (hue / 360) * plot;
}

/** How far from a line, in CSS pixels, the pointer can be and still find it: above or below it, before its start and past its end, where a living one has its dot. */
const REACH = { across: 7, before: 4, after: 10 } as const;

/** The line nearest `point` within reach, or `null`. */
export function lineAt(drawn: readonly AlleleLine[], layout: ChartLayout, point: { x: number; y: number }): AlleleLine | null {
  let nearest: AlleleLine | null = null;
  let distance: number = REACH.across;
  for (const line of drawn) {
    const start = chartX(layout, line.origin);
    const end = chartX(layout, line.lastCarried);
    if (point.x < start - REACH.before || point.x > end + REACH.after) continue;
    const apart = Math.abs(point.y - chartY(layout, line.hue));
    if (apart < distance) {
      distance = apart;
      nearest = line;
    }
  }
  return nearest;
}

/** The alleleLines in the order the keyboard steps through them: down the chart by hue, then by origin. */
export function byHue(drawn: readonly AlleleLine[]): AlleleLine[] {
  return [...drawn].sort((a, b) => a.hue - b.hue || a.origin - b.origin || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
}

/** A line in words: its colour name, where it came from, and how it stands now. */
export function describeLine(line: AlleleLine): { name: string; origin: string; now: string } {
  const origin =
    line.sourceHue !== null
      ? `Mutated from ${colourName(line.sourceHue)} at gen ${line.origin.toLocaleString()}`
      : line.origin === 0
        ? 'A founding colour gene'
        : `Added at gen ${line.origin.toLocaleString()}`;
  const share = `${Math.max(1, Math.round(100 * line.share))}% of colour genes`;
  const now =
    line.status === 'extinct'
      ? `Died out after gen ${line.lastCarried.toLocaleString()}`
      : line.status === 'hidden'
        ? `${share}, hidden everywhere`
        : `${share}, shown by ${line.shown.toLocaleString()} cells`;
  return { name: colourName(line.hue), origin, now };
}
