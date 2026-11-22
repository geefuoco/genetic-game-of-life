import { CARRYING, transferable } from '../protocol.ts';
import type {
  AlleleCensus,
  AlleleId,
  AlleleNode,
  AlleleStatus,
  Census,
  Command,
  Genotype,
  Highlight,
  InspectedAllele,
  Inspection,
  Pedigree,
  PedigreeOrigin,
  PedigreeParent,
  MutationSettings,
  Rate,
  SnapshotPurpose,
  SoupSettings,
  WorkerEvent,
  WorldSource,
} from '../protocol.ts';

/** The World as the engine drives it; in the worker, the WASM `WebWorld`. */
export interface EngineWorld {
  step(generations: number): void;
  generation(): number;
  population(): number;
  width(): number;
  height(): number;
  phenotypes(out: Float32Array): void;
  /** The running Rule in canonical B/S notation. */
  rule(): string;
  /** Switches to a Rule in B/S notation and returns it in canonical notation; throws the core's explanation if it is invalid. */
  setRule(rule: string): string;
  /** Switches the Mutation settings; throws the core's explanation if they are out of range. */
  setMutation(rate: number, magnitude: number): void;
  /** Probability that each Allele passed on mutates. */
  mutationRate(): number;
  /** Largest hue shift of a mutant, in degrees. */
  mutationMagnitude(): number;
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
  /** The census of living Alleles, packed as the WASM `WebWorld` documents. */
  census(): Float64Array;
  /** The Allele Tree, packed as the WASM `WebWorld` documents. */
  alleleTree(): Float64Array;
  /** Writes the Carrier highlight of an Allele into `out` as {@link CARRYING} numbers, all `none` once the World no longer retains it. */
  carryingCells(allele: AlleleId, out: Uint8Array): void;
  /** The population at each Generation from 0 to the current one. */
  populationHistory(): Uint32Array;
  /** The core's Snapshot of the World as it stands. */
  snapshot(): Uint8Array;
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
  /** The World a Snapshot holds; throws the core's explanation if it cannot be imported. */
  importWorld(snapshot: Uint8Array): EngineWorld;
  /** The core's Share Link of a Soup. */
  shareLink(soup: SoupSettings): string;
  /** The Soup a Share Link describes; throws the core's explanation if the core cannot decode it. */
  decodeShareLink(link: string): SoupSettings;
  clock: Clock;
  send(event: WorkerEvent, transfer: Transferable[]): void;
}

/** Longest the engine computes before yielding so commands are handled, in ms. */
const SLICE_MS = 16;
/** Phenotype buffers in circulation between the worker and the page. */
const BUFFERS = 2;
/** Guards against a due Generation being missed through rounding. */
const EPSILON = 1e-9;
/** Shortest time between two censuses, in ms. */
const CENSUS_MS = 250;

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
  /** Carrier highlight buffers the page has handed back, ready to fill. */
  private freeCarriers: Uint8Array[] = [];
  /** The Allele whose Carrier highlight the frames carry, if any. */
  private highlighted: AlleleId | null = null;
  private framePending = false;
  private running = false;
  private rate: Rate = 15;
  private timer: number | null = null;
  /** When the current pace began, and how many Generations have run since. */
  private paceStart = 0;
  private paceDone = 0;
  /** The memory budget of Recorded History each World gets, or `null` for the core's default. */
  private pedigreeBudget: number | null = null;
  private watchingCensus = false;
  private censusSentAt = -Infinity;
  /** The timer for a census held back so censuses stay {@link CENSUS_MS} apart. */
  private censusTimer: number | null = null;
  /** The Soup whose run the World is, while a Share Link can reproduce it. */
  private linkedSoup: SoupSettings | null = null;
  /** The Share Link of {@link linkedSoup}, as last sent to the page. */
  private link: string | null = null;

  constructor(options: EngineOptions) {
    this.options = options;
  }

  handle(command: Command): void {
    switch (command.type) {
      case 'start':
        this.install(() => this.createSoup(command.soup), { source: 'soup', linkedSoup: command.soup, rate: command.rate, running: command.running });
        break;
      case 'open-link':
        this.openLink(command.link, command.rate, command.running);
        break;
      case 'import':
        this.importSnapshot(command);
        break;
      case 'take-snapshot':
        this.takeSnapshot(command.purpose);
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
        this.setMutation(command.mutation);
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
      case 'watch-census':
        this.watchingCensus = command.watching;
        this.cancelCensusTimer();
        if (command.watching) this.sendCensus();
        break;
      case 'highlight':
        this.highlighted = command.allele;
        this.publish();
        break;
      case 'release':
        if (command.carriers?.length === this.cells) this.freeCarriers.push(command.carriers);
        if (command.phenotypes.length === this.cells) {
          this.free.push(command.phenotypes);
          if (this.framePending) this.publish();
        }
        break;
    }
  }

  /** A Soup's World with its Rule and Mutation settings applied. */
  private createSoup(soup: SoupSettings): EngineWorld {
    const world = this.options.createWorld(soup);
    try {
      world.setRule(soup.rule);
      world.setMutation(soup.mutation.rate, soup.mutation.magnitude);
    } catch (error) {
      world.free();
      throw error;
    }
    return world;
  }

  private openLink(link: string, rate: Rate, running: boolean): void {
    let soup: SoupSettings;
    try {
      soup = this.options.decodeShareLink(link);
    } catch (error) {
      this.options.send({ type: 'rejected', source: 'link', message: messageOf(error) }, []);
      return;
    }
    this.install(() => this.createSoup(soup), { source: 'link', linkedSoup: soup, rate, running });
  }

  private importSnapshot(command: Extract<Command, { type: 'import' }>): void {
    this.install(() => this.options.importWorld(command.snapshot), {
      source: command.source,
      linkedSoup: this.decodedOrNull(command.link),
      rate: command.rate,
      running: command.running,
    });
  }

  /** The Soup a Share Link that came with a Snapshot describes, or `null` if there is none or it cannot be decoded. */
  private decodedOrNull(link: string | null): SoupSettings | null {
    if (link === null) return null;
    try {
      return this.options.decodeShareLink(link);
    } catch {
      return null;
    }
  }

  /**
   * Replaces the World with the one `create` makes, once it is ready. If `create` throws, the World is left as it was
   * and the failure is reported: as an error for a new Soup, as a rejection for anything the visitor brought in.
   */
  private install(create: () => EngineWorld, next: { source: WorldSource; linkedSoup: SoupSettings | null; rate: Rate; running: boolean }): void {
    let world: EngineWorld | null = null;
    try {
      world = create();
      if (this.pedigreeBudget !== null) world.setPedigreeBudget(this.pedigreeBudget);
    } catch (error) {
      world?.free();
      const message = messageOf(error);
      this.options.send(next.source === 'soup' ? { type: 'error', message } : { type: 'rejected', source: next.source, message }, []);
      return;
    }
    this.pause();
    this.world?.free();
    this.world = world;
    this.free = [];
    this.freeCarriers = [];
    this.highlighted = null;
    this.framePending = false;
    this.cancelCensusTimer();
    this.censusSentAt = -Infinity;
    this.width = world.width();
    this.height = world.height();
    this.cells = this.width * this.height;
    this.free = Array.from({ length: BUFFERS }, () => new Float32Array(this.cells));
    this.rate = next.rate;
    this.options.send({ type: 'world', source: next.source, rule: world.rule(), mutation: mutationOf(world) }, []);
    this.setLinkedSoup(next.linkedSoup && { ...next.linkedSoup, rule: world.rule(), mutation: mutationOf(world) });
    this.publish();
    if (next.running) {
      this.running = true;
      this.restartPace();
    }
  }

  /** Records the Soup whose run the World is, or `null` once a Share Link cannot reproduce it, and sends a changed Share Link. */
  private setLinkedSoup(soup: SoupSettings | null): void {
    this.linkedSoup = soup;
    let link: string | null = null;
    if (soup) {
      try {
        link = this.options.shareLink(soup);
      } catch {
        this.linkedSoup = null;
      }
    }
    if (link === this.link) return;
    this.link = link;
    this.options.send({ type: 'share-link', link }, []);
  }

  /** Keeps the Share Link through a change of the Rule or Mutation settings at Generation 0, and drops it after: a Share Link only describes how a run starts. */
  private settingChanged(world: EngineWorld): void {
    if (!this.linkedSoup) return;
    const { rule, mutation } = this.linkedSoup;
    const current = mutationOf(world);
    if (rule === world.rule() && mutation.rate === current.rate && mutation.magnitude === current.magnitude) return;
    this.setLinkedSoup(world.generation() === 0 ? { ...this.linkedSoup, rule: world.rule(), mutation: current } : null);
  }

  private setRule(rule: string): void {
    const world = this.world;
    if (!world) return;
    let running: string;
    try {
      running = world.setRule(rule);
    } catch (error) {
      this.options.send({ type: 'rule-rejected', rule, message: messageOf(error) }, []);
      return;
    }
    this.options.send({ type: 'rule', rule: running }, []);
    this.settingChanged(world);
  }

  private setMutation(mutation: MutationSettings): void {
    const world = this.world;
    if (!world) return;
    world.setMutation(mutation.rate, mutation.magnitude);
    this.settingChanged(world);
  }

  /** Edits the World as {@link change} does; a Share Link cannot reproduce an edited World. */
  private edit(apply: (world: EngineWorld) => void): void {
    if (!this.world) return;
    this.setLinkedSoup(null);
    this.change(apply);
  }

  private takeSnapshot(purpose: SnapshotPurpose): void {
    const world = this.world;
    if (!world) return;
    const snapshot = world.snapshot();
    this.options.send({ type: 'snapshot', purpose, snapshot, generation: world.generation(), link: this.link }, [snapshot.buffer]);
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
    if (ran > 0) this.publish();
  }

  /** Sends a census of the current state if one is watched, held back until {@link CENSUS_MS} after the last. */
  private scheduleCensus(): void {
    if (!this.watchingCensus || this.censusTimer !== null) return;
    const wait = this.censusSentAt + CENSUS_MS - this.options.clock.now();
    if (wait <= 0) {
      this.sendCensus();
      return;
    }
    this.censusTimer = this.options.clock.setTimeout(() => {
      this.censusTimer = null;
      this.sendCensus();
    }, wait);
  }

  private cancelCensusTimer(): void {
    if (this.censusTimer !== null) {
      this.options.clock.clearTimeout(this.censusTimer);
      this.censusTimer = null;
    }
  }

  private sendCensus(): void {
    const world = this.world;
    if (!world) return;
    let census: Census;
    try {
      census = unpackCensus(world.census(), world.alleleTree(), world.populationHistory());
    } catch (error) {
      this.options.send({ type: 'error', message: messageOf(error) }, []);
      return;
    }
    this.censusSentAt = this.options.clock.now();
    this.options.send(census, [census.populationHistory.buffer]);
  }

  /** Sends the current state if the page has handed back a buffer, otherwise as soon as it does, and a census if one is due. */
  private publish(): void {
    this.scheduleCensus();
    const world = this.world;
    const buffer = this.free.pop();
    if (!world || !buffer) {
      this.framePending = world !== null;
      return;
    }
    this.framePending = false;
    world.phenotypes(buffer);
    const highlight = this.highlight(world);
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
        highlight,
      },
      transferable({ phenotypes: buffer, carriers: highlight?.carriers ?? null }),
    );
  }

  /** The Carrier highlight asked for, in a handed-back buffer or a new one. */
  private highlight(world: EngineWorld): Highlight | null {
    if (this.highlighted === null) return null;
    const carriers = this.freeCarriers.pop() ?? new Uint8Array(this.cells);
    world.carryingCells(this.highlighted, carriers);
    let shown = 0;
    let hidden = 0;
    for (const mark of carriers) {
      if (mark === CARRYING.shows) shown++;
      else if (mark === CARRYING.hides) hidden++;
    }
    return { allele: this.highlighted, carriers, shown, hidden };
  }
}

function mutationOf(world: EngineWorld): MutationSettings {
  return { rate: world.mutationRate(), magnitude: world.mutationMagnitude() };
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
  const next = reader(packed, 'inspection');
  const [x, y, birthGeneration, deathGeneration, phenotype] = [next(), next(), next(), next(), next()];
  const allele = (): InspectedAllele => {
    const shown = next() === 1;
    const id = alleleId(next(), next());
    const lineage = Array.from({ length: next() }, () => ({
      hue: next(),
      dominance: next(),
      origin: next(),
      status: status(next()),
    }));
    const own = lineage[0];
    if (!own) throw new Error('a packed inspection has an Allele with an empty lineage');
    return { id, hue: own.hue, dominance: own.dominance, shown, lineage };
  };
  const alleles = [allele(), allele()] as const;
  return { x, y, birthGeneration, deathGeneration: deathGeneration < 0 ? null : deathGeneration, phenotype, alleles };
}

/** A census and Allele Tree as the World packs them, unpacked, with the population history; throws if they do not follow the packed layouts. */
function unpackCensus(packed: Float64Array, packedTree: Float64Array, populationHistory: Uint32Array): Census {
  const next = reader(packed, 'census');
  const [generation, population] = [next(), next()];
  const alleles: AlleleCensus[] = [];
  while (next.more()) {
    alleles.push({ id: alleleId(next(), next()), hue: next(), dominance: next(), copies: next(), shown: next(), hidden: next() });
  }
  return { type: 'census', generation, population, alleles, tree: unpackTree(packedTree), populationHistory };
}

/** An Allele Tree as the World packs it, unpacked; throws if it does not follow the packed layout. */
function unpackTree(packed: Float64Array): AlleleNode[] {
  const next = reader(packed, 'Allele Tree');
  const tree: AlleleNode[] = [];
  while (next.more()) {
    const id = alleleId(next(), next());
    const [sourceHigh, sourceLow] = [next(), next()];
    tree.push({
      id,
      source: sourceHigh < 0 ? null : alleleId(sourceHigh, sourceLow),
      hue: next(),
      dominance: next(),
      origin: next(),
      lastCarried: next(),
      status: status(next()),
    });
  }
  return tree;
}

/** Reads the numbers of a packed `what` in order, throwing if it ends early. */
function reader(packed: Float64Array, what: string): (() => number) & { more(): boolean } {
  let at = 0;
  const next = (): number => {
    const value = packed[at++];
    if (value === undefined) throw new Error(`a packed ${what} ends after ${packed.length} numbers`);
    return value;
  };
  return Object.assign(next, { more: () => at < packed.length });
}

/** An Allele id from the high and low 32 bits the World packs it as. */
function alleleId(high: number, low: number): AlleleId {
  return (BigInt(high) << 32n) | BigInt(low);
}

/** Pedigree origins by the number the World packs them as. */
const ORIGINS: readonly PedigreeOrigin[] = ['born', 'placed', 'beyond', 'unexplored'];

/** A Pedigree as the World packs it, unpacked; throws if it does not follow the packed layout. */
function unpackPedigree(packed: Float64Array | undefined): Pedigree | null {
  if (!packed) return null;
  const next = reader(packed, 'pedigree');
  const organisms: Pedigree['organisms'] = [];
  while (next.more()) {
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
