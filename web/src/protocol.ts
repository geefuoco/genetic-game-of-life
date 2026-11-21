/**
 * Messages between the page and the simulation worker. The page sends
 * {@link Command}s; the worker answers with {@link WorkerEvent}s. Phenotype
 * buffers are transferred, never copied: the worker sends one with each
 * frame and the page hands it back with a `release` once it has drawn it.
 */

/** How often and how far Mutation changes an Allele as it is passed on. */
export interface MutationSettings {
  /** Probability, from 0 to 1, that each Allele passed on mutates. */
  rate: number;
  /** Largest hue shift of a mutant, in degrees from 0 to 180. */
  magnitude: number;
}

/** Everything that fixes a Soup run. */
export interface SoupSettings {
  seed: bigint;
  /** The Rule in B/S notation. */
  rule: string;
  /** World width in Cells. */
  width: number;
  /** World height in Cells. */
  height: number;
  /** Probability that a Cell starts live. */
  density: number;
  foundingAlleles: number;
  mutation: MutationSettings;
}

/** One variant of the Pigment Gene. */
export interface Allele {
  /** Hue in degrees, from 0 up to but not including 360. */
  hue: number;
  /** Dominance Level, a whole number from 0 to 3. */
  dominance: number;
}

/** The pair of Alleles a live Cell carries. */
export type Genotype = readonly [Allele, Allele];

/** A Cell's column and row. */
export type CellPosition = readonly [x: number, y: number];

/** The Patterns the core's library can stamp, by the core's names. */
export type PatternName = 'glider' | 'lwss' | 'r-pentomino' | 'gosper-gun';

/** Generations per second, or `'max'` for as fast as the worker can go. */
export type Rate = number | 'max';

export type Command =
  | { type: 'start'; soup: SoupSettings; rate: Rate; running: boolean }
  | { type: 'play' }
  | { type: 'pause' }
  | { type: 'step' }
  | { type: 'set-rate'; rate: Rate }
  | { type: 'set-rule'; rule: string }
  | { type: 'set-mutation'; mutation: MutationSettings }
  /** Makes `cells` live with `genotype`, as part of the open edit stroke. */
  | { type: 'paint'; cells: readonly CellPosition[]; genotype: Genotype }
  /** Makes `cells` dead, as part of the open edit stroke. */
  | { type: 'erase'; cells: readonly CellPosition[] }
  /** Stamps a Pattern centred on a Cell, as part of the open edit stroke. */
  | { type: 'stamp'; pattern: PatternName; x: number; y: number; genotype: Genotype }
  /** Closes the open edit stroke; undo and redo act on whole strokes. */
  | { type: 'end-stroke' }
  | { type: 'undo' }
  | { type: 'redo' }
  | { type: 'release'; phenotypes: Float32Array };

/** One drawable state of the World. */
export interface Frame {
  type: 'frame';
  generation: number;
  population: number;
  width: number;
  height: number;
  /** One Phenotype hue per Cell in row-major order; dead Cells are negative. */
  phenotypes: Float32Array;
  /** Whether an edit stroke of this Generation can be undone. */
  canUndo: boolean;
  /** Whether an undone edit stroke can be redone. */
  canRedo: boolean;
}

export type WorkerEvent =
  | Frame
  | { type: 'error'; message: string }
  /** The Rule now running, in canonical B/S notation, after a `set-rule`. */
  | { type: 'rule'; rule: string }
  /** A `set-rule` the core refused, with its explanation; the running Rule is unchanged. */
  | { type: 'rule-rejected'; rule: string; message: string };
