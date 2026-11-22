/** What the Allele Tree view draws of a census: each Allele's hue over time, from its origin to when it was last carried. */

import type { AlleleId, AlleleStatus, Census } from '../protocol.ts';
import { colourName } from './colours.ts';

/** Living Alleles that are major lineages by rank, most carried first. */
const MAJOR_RANK = 16;
/** Share of the Allele copies from which a living Allele is a major lineage whatever its rank. */
const MAJOR_SHARE = 0.01;

/** One Allele as the Allele Tree view draws it. */
export interface Lineage {
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
  /** Whether it is among the most carried living Alleles, or carries a large share, or is an ancestor of one that does. */
  major: boolean;
}

/** Every Allele in the census's Allele Tree, each before its descendants. */
export function lineages(census: Census): Lineage[] {
  const copies = Math.max(1, 2 * census.population);
  const counts = new Map(census.alleles.map((line) => [line.id, line]));
  const hues = new Map(census.tree.map((node) => [node.id, node.hue]));
  const drawn = census.tree.map((node): Lineage => {
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
  const byId = new Map(drawn.map((lineage) => [lineage.id, lineage]));
  const ranked = [...census.alleles].sort((a, b) => b.copies - a.copies);
  ranked.forEach((line, rank) => {
    if (rank >= MAJOR_RANK && line.copies / copies < MAJOR_SHARE) return;
    for (let at = byId.get(line.id); at && !at.major; at = at.source === null ? undefined : byId.get(at.source)) at.major = true;
  });
  return drawn;
}

/** The Allele `id` and each Allele it descends from, back to its Founding Allele. */
export function ancestry(drawn: readonly Lineage[], id: AlleleId): Set<AlleleId> {
  const byId = new Map(drawn.map((lineage) => [lineage.id, lineage]));
  const chain = new Set<AlleleId>();
  for (let at = byId.get(id); at && !chain.has(at.id); at = at.source === null ? undefined : byId.get(at.source)) chain.add(at.id);
  return chain;
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
export function lineageAt(drawn: readonly Lineage[], layout: ChartLayout, point: { x: number; y: number }): Lineage | null {
  let nearest: Lineage | null = null;
  let distance: number = REACH.across;
  for (const lineage of drawn) {
    const start = chartX(layout, lineage.origin);
    const end = chartX(layout, lineage.lastCarried);
    if (point.x < start - REACH.before || point.x > end + REACH.after) continue;
    const apart = Math.abs(point.y - chartY(layout, lineage.hue));
    if (apart < distance) {
      distance = apart;
      nearest = lineage;
    }
  }
  return nearest;
}

/** The lineages in the order the keyboard steps through them: down the chart by hue, then by origin. */
export function byHue(drawn: readonly Lineage[]): Lineage[] {
  return [...drawn].sort((a, b) => a.hue - b.hue || a.origin - b.origin || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
}

/** A lineage in words: its colour name, where it came from, and how it stands now. */
export function describeLineage(lineage: Lineage): { name: string; origin: string; now: string } {
  const origin =
    lineage.sourceHue !== null
      ? `Mutated from ${colourName(lineage.sourceHue)} at gen ${lineage.origin.toLocaleString()}`
      : lineage.origin === 0
        ? 'A founding colour gene'
        : `Painted in at gen ${lineage.origin.toLocaleString()}`;
  const share = `${Math.max(1, Math.round(100 * lineage.share))}% of colour genes`;
  const now =
    lineage.status === 'extinct'
      ? `Died out after gen ${lineage.lastCarried.toLocaleString()}`
      : lineage.status === 'hidden'
        ? `${share}, hidden everywhere`
        : `${share}, shown by ${lineage.shown.toLocaleString()} cells`;
  return { name: colourName(lineage.hue), origin, now };
}
