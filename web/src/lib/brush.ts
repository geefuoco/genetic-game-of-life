/** The editing tools and the brush they paint and stamp with. */

import type { Allele, Genotype, PatternName } from '../protocol.ts';

/**
 * What a press or drag on the World does: look around, paint live Cells,
 * paint Carriers with the Carrier brush, erase, or stamp a Pattern.
 */
export type Tool = 'look' | 'paint' | 'carrier' | 'erase' | 'stamp';

/** The brush's two Alleles, and the Pattern the stamp tool stamps. */
export interface Brush {
  /** The Allele painted and stamped Cells carry, twice over except with the Carrier brush. */
  first: Allele;
  /** The second Allele the Carrier brush paints. */
  second: Allele;
  pattern: PatternName;
}

/** Highest Dominance Level, as in the core. */
export const MAX_DOMINANCE = 3;

/** An orange that shows over most Alleles, with a weakest blue for Carriers to hide. */
export const DEFAULT_BRUSH: Brush = {
  first: { hue: 20, dominance: 2 },
  second: { hue: 230, dominance: 0 },
  pattern: 'glider',
};

/** The Pattern library, as the stamp tool offers it. */
export const PATTERNS: readonly { name: PatternName; label: string }[] = [
  { name: 'glider', label: 'Glider' },
  { name: 'lwss', label: 'Lightweight spaceship' },
  { name: 'r-pentomino', label: 'R-pentomino' },
  { name: 'gosper-gun', label: 'Gosper glider gun' },
];

/** The Genotype `tool` paints or stamps: both of the brush's Alleles with the Carrier brush, otherwise the first twice. */
export function toolGenotype(tool: Tool, brush: Brush): Genotype {
  return [brush.first, tool === 'carrier' ? brush.second : brush.first];
}

const DOMINANCE_LABELS = ['weakest', 'weak', 'strong', 'strongest'];

/** A Dominance Level as the brush shows it, such as "strong". */
export function dominanceLabel(dominance: number): string {
  return DOMINANCE_LABELS[dominance] ?? String(dominance);
}
