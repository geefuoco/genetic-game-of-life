import type { AlleleStatus, Command, Genotype, InspectedAllele, Inspection, Rate, SoupSettings, WorkerEvent } from '../protocol.ts';

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
  /** Follows the Organism at a Cell for the inspector and returns its packed inspection, or nothing if the Cell is dead. */
  inspect(x: number, y: number): Float64Array | undefined;
  /** The followed Organism's inspection, packed as the WASM `WebWorld` documents, if any. */
  inspection(): Float64Array | undefined;
  stopInspecting(): void;
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
        this.edit((world) => {
          for (const [x, y] of command.cells) world.paint(x, y, genotype);
        });
        break;
      }
      case 'erase':
        this.edit((world) => {
          for (const [x, y] of command.cells) world.erase(x, y);
        });
        break;
      case 'stamp':
        this.edit((world) => world.stamp(command.pattern, command.x, command.y, packed(command.genotype)));
        break;
      case 'end-stroke':
        this.world?.endStroke();
        break;
      case 'undo':
        this.edit((world) => world.undo());
        break;
      case 'redo':
        this.edit((world) => world.redo());
        break;
      case 'inspect':
        this.edit((world) => world.inspect(command.x, command.y));
        break;
      case 'stop-inspecting':
        this.edit((world) => world.stopInspecting());
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

  /** Changes the World between Generations and shows the result, reporting a change the core refuses. */
  private edit(change: (world: EngineWorld) => void): void {
    if (!this.world) return;
    try {
      change(this.world);
    } catch (error) {
      this.options.send({ type: 'error', message: messageOf(error) }, []);
    }
    this.publish();
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

const STATUSES: readonly AlleleStatus[] = ['living', 'hidden', 'extinct'];

/** An inspection as the World packs it, unpacked. */
function unpackInspection(packed: Float64Array | undefined): Inspection | null {
  if (!packed) return null;
  let at = 0;
  const next = (): number => packed[at++] ?? Number.NaN;
  const [x, y, birthGeneration, deathGeneration, phenotype] = [next(), next(), next(), next(), next()];
  const allele = (): InspectedAllele => {
    const shown = next() === 1;
    const lineage = Array.from({ length: next() }, () => ({
      hue: next(),
      dominance: next(),
      origin: next(),
      status: STATUSES[next()] ?? 'extinct',
    }));
    const { hue, dominance } = lineage[0] ?? { hue: Number.NaN, dominance: Number.NaN };
    return { hue, dominance, shown, lineage };
  };
  const alleles = [allele(), allele()] as const;
  return { x, y, birthGeneration, deathGeneration: deathGeneration < 0 ? null : deathGeneration, phenotype, alleles };
}

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
