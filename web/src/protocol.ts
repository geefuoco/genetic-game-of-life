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

/** Where an Allele stands in the current population: shown by a live Cell, carried but shown by none, or carried by none. */
export type AlleleStatus = 'living' | 'hidden' | 'extinct';

/** One Allele in a lineage. */
export interface LineageAllele extends Allele {
  /** The Generation the Allele first appeared in. */
  origin: number;
  status: AlleleStatus;
}

/** One Allele of an inspected Organism. */
export interface InspectedAllele extends Allele {
  /** Whether the Organism's Phenotype shows this Allele; a blend shows both. */
  shown: boolean;
  /** This Allele, then each Allele it descends from, back to its Founding Allele. */
  lineage: LineageAllele[];
}

/** An Organism followed by the inspector: one continuous life at a Cell. */
export interface Inspection {
  x: number;
  y: number;
  /** The Generation of its Birth, or of its painting or stamping. */
  birthGeneration: number;
  /** The first Generation it was no longer live in, or `null` while it lives. */
  deathGeneration: number | null;
  /** Its Phenotype hue. */
  phenotype: number;
  /** Its Genotype. */
  alleles: readonly [InspectedAllele, InspectedAllele];
}

/**
 * How an Organism in a Pedigree came to be: Born from Parents, placed (painted, stamped or live in the World a run started
 * as, so it has no Parents), beyond Recorded History, or Born but at the requested depth, so its Parents were not asked for.
 */
export type PedigreeOrigin = 'born' | 'placed' | 'beyond' | 'unexplored';

/** A Parent in a Pedigree. */
export interface PedigreeParent {
  /** The Parent's index in {@link Pedigree.organisms}. */
  organism: number;
  /** Which of the Parent's two Alleles it passed on. */
  allele: 0 | 1;
}

/** One Organism in a Pedigree. */
export interface PedigreeOrganism {
  x: number;
  y: number;
  /** Its Birth Generation, or `null` when it is beyond Recorded History and no longer live. */
  birthGeneration: number | null;
  /** The fewest Births between it and the inspected Organism: 0 for that Organism, 1 for its Parents. */
  depth: number;
  origin: PedigreeOrigin;
  /** Its Parents, when it was Born and they were asked for. */
  parents: PedigreeParent[];
  /** How many of its Alleles a Birth drew from the Allele Pool rather than a Parent. */
  fromAllelePool: number;
}

/** An inspected Organism's Pedigree: the Organism first, then its ancestors by depth, each shared ancestor once. */
export interface Pedigree {
  organisms: PedigreeOrganism[];
}

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
  /** Follows the Organism at a Cell in the frames' inspection, or nothing if the Cell is dead. */
  | { type: 'inspect'; x: number; y: number }
  | { type: 'stop-inspecting' }
  /** Asks for the inspected Organism's Pedigree, `depth` Births back. */
  | { type: 'pedigree'; depth: number }
  /** Sets the memory budget of Recorded History, kept for later Soups too. */
  | { type: 'set-pedigree-budget'; bytes: number }
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
  /** The Organism being inspected, as of this frame's Generation. */
  inspection: Inspection | null;
}

export type WorkerEvent =
  | Frame
  | { type: 'error'; message: string }
  /** The inspected Organism's Pedigree after a `pedigree` command, or `null` if nothing is inspected. */
  | { type: 'pedigree'; pedigree: Pedigree | null; depth: number }
  /** The Rule now running, in canonical B/S notation, after a `set-rule`. */
  | { type: 'rule'; rule: string }
  /** A `set-rule` the core refused, with its explanation; the running Rule is unchanged. */
  | { type: 'rule-rejected'; rule: string; message: string };
