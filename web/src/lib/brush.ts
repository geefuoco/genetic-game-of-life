/** The editing tools and the brush they paint and stamp with. */

import type { Allele, Genotype, PatternName } from '../protocol.ts';

/** What a press or drag on the World does: look around, paint, erase or stamp a Pattern. */
export type Tool = 'look' | 'paint' | 'erase' | 'stamp';

/** The brush's Alleles, and the Pattern the stamp tool stamps. */
export interface Brush {
  /** The Allele every painted Organism shows unless the Carrier brush gives it a stronger one. */
  colour: Allele;
  /** Whether to paint Carriers of `carried` rather than homozygous Organisms. */
  carrier: boolean;
  /** The second Allele the Carrier brush paints. */
  carried: Allele;
  pattern: PatternName;
}

/** Highest Dominance Level, as in the core. */
export const MAX_DOMINANCE = 3;

/** An orange that shows over most Alleles, with a weakest blue to carry hidden. */
export const DEFAULT_BRUSH: Brush = {
  colour: { hue: 20, dominance: 2 },
  carrier: false,
  carried: { hue: 230, dominance: 0 },
  pattern: 'glider',
};

/** The Pattern library, as the stamp tool offers it. */
export const PATTERNS: readonly { name: PatternName; label: string }[] = [
  { name: 'glider', label: 'Glider' },
  { name: 'lwss', label: 'Lightweight spaceship' },
  { name: 'r-pentomino', label: 'R-pentomino' },
  { name: 'gosper-gun', label: 'Gosper glider gun' },
];

/** The Genotype the brush paints: homozygous, or a Carrier with the Carrier brush. */
export function brushGenotype(brush: Brush): Genotype {
  return [brush.colour, brush.carrier ? brush.carried : brush.colour];
}

const DOMINANCE_LABELS = ['weakest', 'weak', 'strong', 'strongest'];

/** A Dominance Level as the brush shows it, such as "strong". */
export function dominanceLabel(dominance: number): string {
  return DOMINANCE_LABELS[dominance] ?? String(dominance);
}
