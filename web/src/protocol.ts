/**
 * Messages between the page and the simulation worker. The page sends
 * {@link Command}s; the worker answers with {@link WorkerEvent}s. Phenotype
 * buffers are transferred, never copied: the worker sends one with each
 * frame and the page hands it back with a `release` once it has drawn it.
 */

/** Everything that fixes a Soup run under Conway's Life. */
export interface SoupSettings {
  seed: bigint;
  /** World width in Cells. */
  width: number;
  /** World height in Cells. */
  height: number;
  /** Probability that a Cell starts live. */
  density: number;
  foundingAlleles: number;
}

/** Generations per second, or `'max'` for as fast as the worker can go. */
export type Rate = number | 'max';

export type Command =
  | { type: 'start'; soup: SoupSettings; rate: Rate; running: boolean }
  | { type: 'play' }
  | { type: 'pause' }
  | { type: 'step' }
  | { type: 'set-rate'; rate: Rate }
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
}

export type WorkerEvent = Frame | { type: 'error'; message: string };
