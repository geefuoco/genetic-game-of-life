import type {
  AlleleStatus,
  Command,
  Genotype,
  InspectedAllele,
  Inspection,
  Pedigree,
  PedigreeOrigin,
  PedigreeParent,
  Rate,
  SoupSettings,
  WorkerEvent,
} from '../protocol.ts';

/** The World as the engine drives it; in the worker, the WASM `WebWorld`. */
export interface EngineWorld {
  step(generations: number): void;
  generation(): number;
  population(): number;
  phenotypes(out: Float32Array): void;
  /** Switches to a Rule in B/S notation and returns it in canonical notation; throws the core's explanation if it is invalid. */
  setRule(rule: string): string;
  /** Switches the Mutation settings; throws the core's explanation if they are out of range. */
  setMutation(rate: number, magnitude: number): void;
  /** Paints a Cell with a Genotype given as `[hue, dominance, hue, dominance]`, in the open stroke. */
  paint(x: number, y: number, genotype: Float64Array): void;
  /** Erases a Cell, in the open stroke. */
  erase(x: number, y: number): void;
  /** Stamps a named Pattern centred on a Cell, in the open stroke. */
  stamp(pattern: string, x: number, y: number, genotype: Float64Array): void;
  endStroke(): void;
  undo(): boolean;
  redo(): boolean;
  canUndo(): boolean;
  canRedo(): boolean;
  /** Follows the Organism at a Cell for the inspector, or nothing if the Cell is dead. */
  inspect(x: number, y: number): void;
  /** The followed Organism's inspection, packed as the WASM `WebWorld` documents, if any. */
  inspection(): Float64Array | undefined;
  stopInspecting(): void;
  /** The followed Organism's Pedigree `depth` Births back, packed as the WASM `WebWorld` documents, if any. */
  pedigree(depth: number): Float64Array | undefined;
  /** Sets the memory budget of Recorded History in bytes; throws the core's explanation if it is too large. */
  setPedigreeBudget(bytes: number): void;
  free(): void;
}

/** Time and timers, injected so tests can control them. */
export interface Clock {
  now(): number;
  setTimeout(run: () => void, ms: number): number;
  clearTimeout(id: number): void;
}

export interface EngineOptions {
  /** The World a Soup describes; the engine applies its Rule and Mutation settings after creating it. */
  createWorld(soup: SoupSettings): EngineWorld;
  clock: Clock;
  send(event: WorkerEvent, transfer: Transferable[]): void;
}

/** Longest the engine computes before yielding so commands are handled, in ms. */
const SLICE_MS = 16;
/** Phenotype buffers in circulation between the worker and the page. */
const BUFFERS = 2;
/** Guards against a due Generation being missed through rounding. */
const EPSILON = 1e-9;

/**
 * Runs the simulation for the worker: handles the page's commands, steps the
 * World at the chosen rate, and sends frames whenever the page has handed
 * back a buffer to fill, so a page that falls behind only ever sees the
 * latest Generation.
 */
export class Engine {
  private readonly options: EngineOptions;
  private world: EngineWorld | null = null;
  private cells = 0;
  private width = 0;
  private height = 0;
  private free: Float32Array[] = [];
  private framePending = false;
  private running = false;
  private rate: Rate = 15;
  private timer: number | null = null;
  /** When the current pace began, and how many Generations have run since. */
  private paceStart = 0;
  private paceDone = 0;
  /** The memory budget of Recorded History each World gets, or `null` for the core's default. */
  private pedigreeBudget: number | null = null;

  constructor(options: EngineOptions) {
    this.options = options;
  }

  handle(command: Command): void {
    switch (command.type) {
      case 'start':
        this.start(command.soup, command.rate, command.running);
        break;
      case 'play':
        if (!this.running) {
          this.running = true;
          this.restartPace();
        }
        break;
      case 'pause':
        this.pause();
        break;
      case 'step':
        this.pause();
        if (this.world) {
          this.world.step(1);
          this.publish();
        }
        break;
      case 'set-rate':
        this.rate = command.rate;
        if (this.running) this.restartPace();
        break;
      case 'set-rule':
        this.setRule(command.rule);
        break;
      case 'set-mutation':
        this.world?.setMutation(command.mutation.rate, command.mutation.magnitude);
        break;
      case 'paint': {
        const genotype = packed(command.genotype);
        this.change((world) => {
          for (const [x, y] of command.cells) world.paint(x, y, genotype);
        });
        break;
      }
      case 'erase':
        this.change((world) => {
          for (const [x, y] of command.cells) world.erase(x, y);
        });
        break;
      case 'stamp':
        this.change((world) => world.stamp(command.pattern, command.x, command.y, packed(command.genotype)));
        break;
      case 'end-stroke':
        this.world?.endStroke();
        break;
      case 'undo':
        this.change((world) => world.undo());
        break;
      case 'redo':
        this.change((world) => world.redo());
        break;
      case 'inspect':
        this.change((world) => world.inspect(command.x, command.y));
        break;
      case 'stop-inspecting':
        this.change((world) => world.stopInspecting());
        break;
      case 'pedigree':
        this.sendPedigree(command.depth);
        break;
      case 'set-pedigree-budget':
        this.pedigreeBudget = command.bytes;
        this.change((world) => world.setPedigreeBudget(command.bytes));
        break;
      case 'release':
        if (command.phenotypes.length === this.cells) {
          this.free.push(command.phenotypes);
          if (this.framePending) this.publish();
        }
        break;
    }
  }

  private start(soup: SoupSettings, rate: Rate, running: boolean): void {
    this.pause();
    this.world?.free();
    this.world = null;
    this.cells = 0;
    this.free = [];
    this.framePending = false;
    let world: EngineWorld | null = null;
    try {
      world = this.options.createWorld(soup);
      world.setRule(soup.rule);
      world.setMutation(soup.mutation.rate, soup.mutation.magnitude);
      if (this.pedigreeBudget !== null) world.setPedigreeBudget(this.pedigreeBudget);
    } catch (error) {
      world?.free();
      this.options.send({ type: 'error', message: messageOf(error) }, []);
      return;
    }
    this.world = world;
    this.width = soup.width;
    this.height = soup.height;
    this.cells = soup.width * soup.height;
    this.free = Array.from({ length: BUFFERS }, () => new Float32Array(this.cells));
    this.rate = rate;
    this.publish();
    if (running) {
      this.running = true;
      this.restartPace();
    }
  }

  private setRule(rule: string): void {
    if (!this.world) return;
    let running: string;
    try {
      running = this.world.setRule(rule);
    } catch (error) {
      this.options.send({ type: 'rule-rejected', rule, message: messageOf(error) }, []);
      return;
    }
    this.options.send({ type: 'rule', rule: running }, []);
  }

  /** Changes the World, or what it inspects, between Generations and shows the result, reporting a change the core refuses. */
  private change(apply: (world: EngineWorld) => void): void {
    if (!this.world) return;
    try {
      apply(this.world);
    } catch (error) {
      this.options.send({ type: 'error', message: messageOf(error) }, []);
    }
    this.publish();
  }

  private sendPedigree(depth: number): void {
    const world = this.world;
    if (!world) return;
    let pedigree: Pedigree | null;
    try {
      pedigree = unpackPedigree(world.pedigree(depth));
    } catch (error) {
      this.options.send({ type: 'error', message: messageOf(error) }, []);
      return;
    }
    this.options.send({ type: 'pedigree', pedigree, depth }, []);
  }

  private pause(): void {
    this.running = false;
    this.cancelTimer();
  }

  private cancelTimer(): void {
    if (this.timer !== null) {
      this.options.clock.clearTimeout(this.timer);
      this.timer = null;
    }
  }

  private schedule(ms: number): void {
    this.cancelTimer();
    this.timer = this.options.clock.setTimeout(() => {
      this.timer = null;
      this.runSlice();
    }, ms);
  }

  private restartPace(): void {
    this.paceStart = this.options.clock.now();
    this.paceDone = 0;
    this.schedule(0);
  }

  /** Runs the Generations now due, then schedules the next slice. */
  private runSlice(): void {
    const world = this.world;
    if (!world || !this.running) return;
    const { clock } = this.options;
    const begun = clock.now();
    let ran = 0;
    if (this.rate === 'max') {
      do {
        world.step(1);
        ran++;
      } while (clock.now() - begun < SLICE_MS);
      this.schedule(0);
    } else {
      const rate = this.rate;
      const due = Math.floor(((begun - this.paceStart) * rate) / 1000 + EPSILON) - this.paceDone;
      while (ran < due && clock.now() - begun < SLICE_MS) {
        world.step(1);
        ran++;
      }
      this.paceDone += ran;
      if (ran < due) {
        this.paceStart = clock.now();
        this.paceDone = 0;
      }
      const next = this.paceStart + ((this.paceDone + 1) * 1000) / rate;
      this.schedule(next - clock.now());
    }
    if (ran > 0) this.publish();
  }

  /** Sends the current state if the page has handed back a buffer, otherwise as soon as it does. */
  private publish(): void {
    const world = this.world;
    const buffer = this.free.pop();
    if (!world || !buffer) {
      this.framePending = world !== null;
      return;
    }
    this.framePending = false;
    world.phenotypes(buffer);
    this.options.send(
      {
        type: 'frame',
        generation: world.generation(),
        population: world.population(),
        width: this.width,
        height: this.height,
        phenotypes: buffer,
        canUndo: world.canUndo(),
        canRedo: world.canRedo(),
        inspection: unpackInspection(world.inspection()),
      },
      [buffer.buffer],
    );
  }
}

/** A Genotype as the World takes it: `[hue, dominance, hue, dominance]`. */
function packed([first, second]: Genotype): Float64Array {
  return Float64Array.of(first.hue, first.dominance, second.hue, second.dominance);
}

/** Allele statuses by the number the World packs them as. */
const STATUSES: readonly AlleleStatus[] = ['living', 'hidden', 'extinct'];

/** An inspection as the World packs it, unpacked; throws if it does not follow the packed layout. */
function unpackInspection(packed: Float64Array | undefined): Inspection | null {
  if (!packed) return null;
  let at = 0;
  const next = (): number => {
    const value = packed[at++];
    if (value === undefined) throw new Error(`a packed inspection ends after ${packed.length} numbers`);
    return value;
  };
  const [x, y, birthGeneration, deathGeneration, phenotype] = [next(), next(), next(), next(), next()];
  const allele = (): InspectedAllele => {
    const shown = next() === 1;
    const lineage = Array.from({ length: next() }, () => ({
      hue: next(),
      dominance: next(),
      origin: next(),
      status: status(next()),
    }));
    const own = lineage[0];
    if (!own) throw new Error('a packed inspection has an Allele with an empty lineage');
    return { hue: own.hue, dominance: own.dominance, shown, lineage };
  };
  const alleles = [allele(), allele()] as const;
  return { x, y, birthGeneration, deathGeneration: deathGeneration < 0 ? null : deathGeneration, phenotype, alleles };
}

/** Pedigree origins by the number the World packs them as. */
const ORIGINS: readonly PedigreeOrigin[] = ['born', 'placed', 'beyond', 'unexplored'];

/** A Pedigree as the World packs it, unpacked; throws if it does not follow the packed layout. */
function unpackPedigree(packed: Float64Array | undefined): Pedigree | null {
  if (!packed) return null;
  let at = 0;
  const next = (): number => {
    const value = packed[at++];
    if (value === undefined) throw new Error(`a packed pedigree ends after ${packed.length} numbers`);
    return value;
  };
  const organisms: Pedigree['organisms'] = [];
  while (at < packed.length) {
    const [x, y, birthGeneration, depth, origin, fromAllelePool, parentCount] = [next(), next(), next(), next(), next(), next(), next()];
    const parents = Array.from({ length: parentCount }, (): PedigreeParent => ({ organism: next(), allele: next() === 1 ? 1 : 0 }));
    const named = ORIGINS[origin];
    if (!named) throw new Error(`${origin} is not a packed Pedigree origin`);
    organisms.push({ x, y, birthGeneration: birthGeneration < 0 ? null : birthGeneration, depth, origin: named, parents, fromAllelePool });
  }
  return { organisms };
}

function status(packed: number): AlleleStatus {
  const named = STATUSES[packed];
  if (!named) throw new Error(`${packed} is not a packed Allele status`);
  return named;
}

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
