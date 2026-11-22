import { readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { initSync, WebWorld } from '../../wasm-pkg/genetic_life_wasm.js';
import type { Census, Command, Frame, FrameBuffers, Rate, SoupSettings, WorkerEvent } from '../protocol.ts';
import { RULE_PRESETS } from '../lib/settings.ts';
import { Engine, type Clock, type EngineWorld } from './engine.ts';
import { shareLinkOf, soupOfShareLink } from './soup.ts';

beforeAll(() => {
  initSync({ module: readFileSync(new URL('../../wasm-pkg/genetic_life_wasm_bg.wasm', import.meta.url)) });
});

const SOUP: SoupSettings = { seed: 42n, rule: 'B3/S23', width: 160, height: 160, density: 0.3, foundingAlleles: 8, mutation: { rate: 0.01, magnitude: 30 } };

/** Life without Death: every live Cell survives, so the population never falls. */
const LIFE_WITHOUT_DEATH = 'B3/S012345678';

function createWebWorld(soup: SoupSettings): WebWorld {
  return new WebWorld(soup.seed, soup.width, soup.height, soup.density, soup.foundingAlleles);
}

function isNonDecreasing(values: number[]): boolean {
  return values.every((value, i) => i === 0 || value >= (values[i - 1] ?? value));
}

/** A clock whose time only moves when the test, or a costly World, moves it. */
class FakeClock implements Clock {
  time = 0;
  private timers = new Map<number, { at: number; run: () => void }>();
  private nextId = 1;

  now(): number {
    return this.time;
  }

  setTimeout(run: () => void, ms: number): number {
    const id = this.nextId++;
    this.timers.set(id, { at: this.time + Math.max(0, ms), run });
    return id;
  }

  clearTimeout(id: number): void {
    this.timers.delete(id);
  }

  advance(ms: number): void {
    const end = this.time + ms;
    for (;;) {
      const due = [...this.timers.entries()]
        .filter(([, timer]) => timer.at <= end)
        .sort(([a, x], [b, y]) => x.at - y.at || a - b)[0];
      if (!due) break;
      const [id, timer] = due;
      this.timers.delete(id);
      this.time = Math.max(this.time, timer.at);
      timer.run();
    }
    this.time = Math.max(this.time, end);
  }
}

/** The events about where the World came from and how it is kept. */
type Persistence = Extract<WorkerEvent, { type: 'world' | 'rejected' | 'share-link' | 'snapshot' }>;

const PERSISTENCE: readonly WorkerEvent['type'][] = ['world', 'rejected', 'share-link', 'snapshot'];

interface Harness {
  clock: FakeClock;
  frames: Frame[];
  errors: string[];
  /** Events other than frames, errors and {@link Persistence} events, in order. */
  notices: Exclude<WorkerEvent, Frame | Persistence | { type: 'error' }>[];
  /** {@link Persistence} events, in order. */
  persistence: Persistence[];
  send(command: Command): void;
  /** Hands every held buffer back, as a page that keeps up would. */
  releaseAll(): void;
  latest(): Frame;
  /** Simulated time each Generation takes to compute. */
  cost: { msPerGeneration: number };
}

function harness(options: { keepUp?: boolean } = {}): Harness {
  const keepUp = options.keepUp ?? true;
  const clock = new FakeClock();
  const frames: Frame[] = [];
  const errors: string[] = [];
  const notices: Harness['notices'] = [];
  const persistence: Persistence[] = [];
  const held: FrameBuffers[] = [];
  const cost = { msPerGeneration: 0 };
  const adapt = (world: WebWorld): EngineWorld => ({
    step(generations) {
      world.step(generations);
      clock.time += cost.msPerGeneration * generations;
    },
    generation: () => world.generation(),
    population: () => world.population(),
    width: () => world.width(),
    height: () => world.height(),
    phenotypes: (out) => world.phenotypes(out),
    rule: () => world.rule(),
    setRule: (rule) => world.setRule(rule),
    mutationRate: () => world.mutationRate(),
    mutationMagnitude: () => world.mutationMagnitude(),
    setMutation: (rate, magnitude) => world.setMutation(rate, magnitude),
    paint: (x, y, genotype) => world.paint(x, y, genotype),
    erase: (x, y) => world.erase(x, y),
    stamp: (pattern, x, y, genotype) => world.stamp(pattern, x, y, genotype),
    endStroke: () => world.endStroke(),
    undo: () => world.undo(),
    redo: () => world.redo(),
    canUndo: () => world.canUndo(),
    canRedo: () => world.canRedo(),
    inspect: (x, y) => world.inspect(x, y),
    inspection: () => world.inspection(),
    stopInspecting: () => world.stopInspecting(),
    pedigree: (depth) => world.pedigree(depth),
    setPedigreeBudget: (bytes) => world.setPedigreeBudget(bytes),
    census: () => world.census(),
    alleleTree: () => world.alleleTree(),
    carryingCells: (allele, out) => world.carryingCells(allele, out),
    populationHistory: () => world.populationHistory(),
    snapshot: () => world.snapshot(),
    free: () => world.free(),
  });
  const onEvent = (event: WorkerEvent): void => {
    if (event.type === 'error') {
      errors.push(event.message);
      return;
    }
    if (isPersistence(event)) {
      persistence.push(event);
      return;
    }
    if (event.type !== 'frame') {
      notices.push(event);
      return;
    }
    const highlight = event.highlight && { ...event.highlight, carriers: event.highlight.carriers.slice() };
    frames.push({ ...event, phenotypes: event.phenotypes.slice(), highlight });
    const buffers: FrameBuffers = { phenotypes: event.phenotypes, carriers: event.highlight?.carriers ?? null };
    if (keepUp) queueRelease(buffers);
    else held.push(buffers);
  };
  const releases: FrameBuffers[] = [];
  const queueRelease = (buffers: FrameBuffers): void => {
    releases.push(buffers);
    clock.setTimeout(() => {
      const next = releases.shift();
      if (next) engine.handle({ type: 'release', ...next });
    }, 0);
  };
  const engine = new Engine({
    createWorld: (soup) => adapt(createWebWorld(soup)),
    importWorld: (snapshot) => adapt(WebWorld.fromSnapshot(snapshot)),
    shareLink: shareLinkOf,
    decodeShareLink: soupOfShareLink,
    clock,
    send: onEvent,
  });
  return {
    clock,
    frames,
    errors,
    notices,
    persistence,
    cost,
    send: (command) => engine.handle(command),
    releaseAll() {
      for (const buffers of held.splice(0)) engine.handle({ type: 'release', ...buffers });
    },
    latest() {
      const frame = frames.at(-1);
      if (!frame) throw new Error('no frame yet');
      return frame;
    },
  };
}

function isPersistence(event: WorkerEvent): event is Persistence {
  return PERSISTENCE.includes(event.type);
}

/** The Share Links sent, in order. */
function links(h: Harness): (string | null)[] {
  return h.persistence.flatMap((event) => (event.type === 'share-link' ? [event.link] : []));
}

function start(h: Harness, rate: Rate = 15, running = true, soup = SOUP): void {
  h.send({ type: 'start', soup, rate, running });
}

function referencePhenotypes(soup: SoupSettings, generations: number): Float32Array {
  const world = createWebWorld(soup);
  world.setRule(soup.rule);
  world.setMutation(soup.mutation.rate, soup.mutation.magnitude);
  world.step(generations);
  const out = new Float32Array(soup.width * soup.height);
  world.phenotypes(out);
  world.free();
  return out;
}

describe('starting', () => {
  it('sends a first frame of the Soup at Generation 0', () => {
    const h = harness();
    start(h);
    const frame = h.latest();
    expect(frame.generation).toBe(0);
    expect(frame.width).toBe(160);
    expect(frame.height).toBe(160);
    expect(frame.phenotypes).toHaveLength(160 * 160);
    expect(frame.population).toBeGreaterThan(0);
    expect(frame.phenotypes.filter((hue) => hue >= 0)).toHaveLength(frame.population);
  });

  it('reports a Soup the core rejects as an error', () => {
    const h = harness();
    start(h, 15, true, { ...SOUP, width: 8 });
    expect(h.frames).toHaveLength(0);
    expect(h.errors[0]).toMatch(/size 8x160/);
  });

  it('reports a Soup whose Rule the core rejects as an error', () => {
    const h = harness();
    start(h, 15, true, { ...SOUP, rule: 'B9/S23' });
    expect(h.frames).toHaveLength(0);
    expect(h.errors).toEqual(['neighbour count 9 is too large, the maximum is 8']);
  });

  it('replaces the running World when started again', () => {
    const h = harness();
    start(h);
    h.clock.advance(1000);
    start(h, 15, true, { ...SOUP, width: 64, height: 32 });
    h.clock.advance(1000);
    expect(h.latest().width).toBe(64);
    expect(h.latest().generation).toBe(15);
  });
});

describe('pacing', () => {
  it('runs 15 Generations per second at 1x', () => {
    const h = harness();
    start(h, 15);
    h.clock.advance(2000);
    expect(h.latest().generation).toBe(30);
  });

  it('runs 7.5 Generations per second at half speed', () => {
    const h = harness();
    start(h, 7.5);
    h.clock.advance(2000);
    expect(h.latest().generation).toBe(15);
  });

  it('changes pace while running', () => {
    const h = harness();
    start(h, 15);
    h.clock.advance(1000);
    h.send({ type: 'set-rate', rate: 60 });
    h.clock.advance(1000);
    expect(h.latest().generation).toBe(75);
  });

  it('runs Max far faster than the fastest paced speed', () => {
    const h = harness();
    h.cost.msPerGeneration = 1;
    start(h, 'max');
    h.clock.advance(1000);
    expect(h.latest().generation).toBeGreaterThan(500);
  });

  it('runs a World too slow for its pace as fast as it can, without a backlog to catch up', () => {
    const h = harness();
    h.cost.msPerGeneration = 50;
    start(h, 60);
    h.clock.advance(1000);
    const slowEnd = h.latest().generation;
    expect(slowEnd).toBeGreaterThan(10);
    expect(slowEnd).toBeLessThanOrEqual(21);
    h.cost.msPerGeneration = 0;
    h.clock.advance(1000);
    expect(h.latest().generation - slowEnd).toBeLessThanOrEqual(61);
  });
});

describe('play, pause and step', () => {
  it('stays at Generation 0 when started paused', () => {
    const h = harness();
    start(h, 15, false);
    h.clock.advance(1000);
    expect(h.latest().generation).toBe(0);
  });

  it('stops on pause and resumes on play', () => {
    const h = harness();
    start(h, 15);
    h.clock.advance(1000);
    h.send({ type: 'pause' });
    h.clock.advance(1000);
    expect(h.latest().generation).toBe(15);
    h.send({ type: 'play' });
    h.clock.advance(1000);
    expect(h.latest().generation).toBe(30);
  });

  it('stops Max on pause', () => {
    const h = harness();
    h.cost.msPerGeneration = 1;
    start(h, 'max');
    h.clock.advance(100);
    h.send({ type: 'pause' });
    const paused = h.latest().generation;
    h.clock.advance(1000);
    expect(h.latest().generation).toBe(paused);
  });

  it('steps exactly one Generation and pauses', () => {
    const h = harness();
    start(h, 15);
    h.clock.advance(1000);
    h.send({ type: 'step' });
    h.clock.advance(0);
    expect(h.latest().generation).toBe(16);
    h.clock.advance(1000);
    expect(h.latest().generation).toBe(16);
  });
});

describe('frames', () => {
  it('carry the Phenotypes of the Generation they report', () => {
    const h = harness();
    start(h, 15);
    h.clock.advance(1000);
    const frame = h.latest();
    expect(frame.phenotypes).toEqual(referencePhenotypes(SOUP, frame.generation));
  });

  it('are held back while the page holds every buffer, then sent at the latest Generation', () => {
    const h = harness({ keepUp: false });
    start(h, 15);
    h.clock.advance(1000);
    expect(h.frames.map((frame) => frame.generation)).toEqual([0, 1]);
    h.releaseAll();
    expect(h.latest().generation).toBe(15);
  });

  it('ignore buffers sized for a World that has been replaced', () => {
    const h = harness({ keepUp: false });
    start(h, 15, false);
    start(h, 15, false, { ...SOUP, width: 64, height: 32 });
    h.send({ type: 'step' });
    h.releaseAll();
    expect(h.frames.map((frame) => [frame.width, frame.generation])).toEqual([
      [160, 0],
      [64, 0],
      [64, 1],
    ]);
  });
});

describe('the Rule', () => {
  it('changes mid-run without resetting the World', () => {
    const h = harness();
    start(h);
    h.clock.advance(1000);
    h.send({ type: 'set-rule', rule: LIFE_WITHOUT_DEATH });
    const switched = h.frames.length;
    h.clock.advance(1000);
    expect(h.latest().generation).toBe(30);
    expect(isNonDecreasing(h.frames.slice(switched - 1).map((frame) => frame.population))).toBe(true);
  });

  it('confirms the new Rule in canonical notation', () => {
    const h = harness();
    start(h);
    h.send({ type: 'set-rule', rule: 'b63/s32' });
    expect(h.notices).toEqual([{ type: 'rule', rule: 'B36/S23' }]);
  });

  it('accepts every Rule Preset the settings offer, as written', () => {
    const h = harness();
    start(h);
    for (const preset of RULE_PRESETS) h.send({ type: 'set-rule', rule: preset.notation });
    expect(h.notices).toEqual(RULE_PRESETS.map((preset) => ({ type: 'rule', rule: preset.notation })));
  });

  it('rejects invalid B/S notation with the core explanation and keeps the running Rule', () => {
    const h = harness();
    start(h, 15, true, { ...SOUP, rule: LIFE_WITHOUT_DEATH });
    h.send({ type: 'set-rule', rule: 'B3/S2x' });
    const rejected = h.frames.length;
    h.clock.advance(1000);
    expect(h.notices).toEqual([{ type: 'rule-rejected', rule: 'B3/S2x', message: "'x' is not a digit from 0 to 8" }]);
    expect(h.errors).toEqual([]);
    expect(h.latest().generation).toBe(15);
    expect(isNonDecreasing(h.frames.slice(rejected - 1).map((frame) => frame.population))).toBe(true);
  });
});

describe('Mutation', () => {
  /** Phenotypes 8 Founding Alleles can make without Mutation: each Allele alone, or any two blended. */
  const WITHOUT_MUTATION = 8 + (8 * 7) / 2;

  const distinctLiveHues = (frame: Frame): number => new Set(frame.phenotypes.filter((hue) => hue >= 0)).size;

  it('changes mid-run', () => {
    const h = harness();
    start(h, 15, true, { ...SOUP, mutation: { rate: 0, magnitude: 30 } });
    h.clock.advance(1000);
    expect(distinctLiveHues(h.latest())).toBeLessThanOrEqual(WITHOUT_MUTATION);
    h.send({ type: 'set-mutation', mutation: { rate: 1, magnitude: 180 } });
    h.clock.advance(1000);
    expect(h.latest().generation).toBe(30);
    expect(distinctLiveHues(h.latest())).toBeGreaterThan(WITHOUT_MUTATION);
  });
});

describe('editing', () => {
  const EMPTY: SoupSettings = { ...SOUP, width: 32, height: 32, density: 0 };
  const ORANGE = { hue: 40, dominance: 2 };
  const BLUE = { hue: 230, dominance: 0 };
  const hueAt = (frame: Frame, x: number, y: number): number => frame.phenotypes[y * frame.width + x] ?? Number.NaN;

  it('paints Cells with a Genotype and sends a frame showing them', () => {
    const h = harness();
    start(h, 15, false, EMPTY);
    h.send({ type: 'paint', cells: [[3, 4], [4, 4]], genotype: [ORANGE, BLUE] });
    const frame = h.latest();
    expect(frame.generation).toBe(0);
    expect(frame.population).toBe(2);
    expect(hueAt(frame, 3, 4)).toBe(40);
    expect(hueAt(frame, 4, 4)).toBe(40);
    expect(frame.canUndo).toBe(true);
  });

  it('erases Cells', () => {
    const h = harness();
    start(h, 15, false, EMPTY);
    h.send({ type: 'paint', cells: [[3, 4], [4, 4]], genotype: [ORANGE, ORANGE] });
    h.send({ type: 'erase', cells: [[3, 4]] });
    h.clock.advance(0);
    expect(h.latest().population).toBe(1);
    expect(hueAt(h.latest(), 3, 4)).toBeLessThan(0);
  });

  it('stamps a Pattern centred on a Cell', () => {
    const h = harness();
    start(h, 15, false, EMPTY);
    h.send({ type: 'stamp', pattern: 'glider', x: 10, y: 10, genotype: [ORANGE, ORANGE] });
    expect(h.latest().population).toBe(5);
    expect(hueAt(h.latest(), 11, 11)).toBe(40);
  });

  it('undoes and redoes whole strokes', () => {
    const h = harness();
    start(h, 15, false, EMPTY);
    h.send({ type: 'paint', cells: [[1, 1]], genotype: [ORANGE, ORANGE] });
    h.send({ type: 'paint', cells: [[2, 1]], genotype: [ORANGE, ORANGE] });
    h.send({ type: 'end-stroke' });
    h.send({ type: 'undo' });
    h.clock.advance(0);
    expect(h.latest()).toMatchObject({ population: 0, canUndo: false, canRedo: true });
    h.send({ type: 'redo' });
    h.clock.advance(0);
    expect(h.latest()).toMatchObject({ population: 2, canUndo: true, canRedo: false });
  });

  it('forgets the edit history once a Generation advances', () => {
    const h = harness();
    start(h, 15, false, EMPTY);
    h.send({ type: 'paint', cells: [[1, 1]], genotype: [ORANGE, ORANGE] });
    h.send({ type: 'end-stroke' });
    h.send({ type: 'step' });
    h.clock.advance(0);
    expect(h.latest()).toMatchObject({ generation: 1, canUndo: false, canRedo: false });
  });

  it('applies edits while running without disturbing the pace', () => {
    const h = harness();
    start(h, 15, true, EMPTY);
    h.clock.advance(500);
    const generation = h.latest().generation;
    h.send({ type: 'stamp', pattern: 'r-pentomino', x: 16, y: 16, genotype: [ORANGE, ORANGE] });
    h.clock.advance(0);
    expect(h.latest()).toMatchObject({ generation, population: 5 });
    h.clock.advance(1500);
    expect(h.latest().generation).toBe(30);
    expect(h.latest().population).toBeGreaterThan(0);
  });

  it('reports an edit the core refuses as an error', () => {
    const h = harness();
    start(h, 15, false, EMPTY);
    h.send({ type: 'paint', cells: [[40, 0]], genotype: [ORANGE, ORANGE] });
    expect(h.errors).toEqual(['cell (40, 0) is outside the world']);
  });
});

describe('inspecting', () => {
  const EMPTY: SoupSettings = { ...SOUP, width: 32, height: 32, density: 0, mutation: { rate: 0, magnitude: 0 } };
  const ORANGE = { hue: 40, dominance: 2 };
  const JADE = { hue: 160, dominance: 1 };

  it('sends the inspected Organism with each frame: its Genotype, Phenotype, Birth Generation and lineages', () => {
    const h = harness();
    start(h, 15, false, EMPTY);
    h.send({ type: 'step' });
    h.send({ type: 'paint', cells: [[3, 4]], genotype: [JADE, ORANGE] });
    h.send({ type: 'inspect', x: 3, y: 4 });
    h.clock.advance(0);
    expect(h.latest().inspection).toEqual({
      x: 3,
      y: 4,
      birthGeneration: 1,
      deathGeneration: null,
      phenotype: 40,
      alleles: [
        { ...JADE, id: expect.any(BigInt), shown: false, lineage: [{ ...JADE, origin: 1, status: 'hidden' }] },
        { ...ORANGE, id: expect.any(BigInt), shown: true, lineage: [{ ...ORANGE, origin: 1, status: 'living' }] },
      ],
    });
  });

  it('identifies each inspected Allele as the census does', () => {
    const h = harness();
    start(h, 15, false, EMPTY);
    h.send({ type: 'paint', cells: [[3, 4]], genotype: [JADE, ORANGE] });
    h.send({ type: 'inspect', x: 3, y: 4 });
    h.send({ type: 'watch-census', watching: true });
    h.clock.advance(0);
    const census = h.notices.filter((notice): notice is Census => notice.type === 'census').at(-1);
    const ids = h.latest().inspection?.alleles.map((allele) => allele.id);
    expect(ids).toEqual([JADE, ORANGE].map((allele) => census?.alleles.find((line) => line.hue === allele.hue)?.id));
  });

  it('stays with the inspected Organism through its death, not the next Organism Born in its Cell', () => {
    const h = harness();
    start(h, 15, false, EMPTY);
    h.send({ type: 'paint', cells: [[4, 5], [5, 5], [6, 5]], genotype: [ORANGE, ORANGE] });
    h.send({ type: 'end-stroke' });
    h.send({ type: 'inspect', x: 4, y: 5 });
    h.send({ type: 'step' });
    h.send({ type: 'step' });
    h.clock.advance(0);
    expect(h.latest().generation).toBe(2);
    expect(h.latest().phenotypes[5 * 32 + 4]).toBe(40);
    expect(h.latest().inspection).toMatchObject({ x: 4, y: 5, birthGeneration: 0, deathGeneration: 1 });
  });

  it('inspects nothing after a dead Cell is inspected, after inspection stops, or in a new Soup', () => {
    const h = harness();
    start(h, 15, false, EMPTY);
    h.send({ type: 'paint', cells: [[3, 4]], genotype: [ORANGE, ORANGE] });
    h.send({ type: 'inspect', x: 3, y: 4 });
    h.send({ type: 'inspect', x: 9, y: 9 });
    h.clock.advance(0);
    expect(h.latest().inspection).toBeNull();

    h.send({ type: 'inspect', x: 3, y: 4 });
    h.send({ type: 'stop-inspecting' });
    h.clock.advance(0);
    expect(h.latest().inspection).toBeNull();

    h.send({ type: 'inspect', x: 3, y: 4 });
    start(h, 15, false, EMPTY);
    expect(h.latest().inspection).toBeNull();
  });
});

describe('Carrier highlight', () => {
  const EMPTY: SoupSettings = { ...SOUP, width: 32, height: 32, density: 0, mutation: { rate: 0, magnitude: 0 } };
  const ORANGE = { hue: 40, dominance: 2 };
  const JADE = { hue: 160, dominance: 1 };
  const at = (x: number, y: number): number => y * 32 + x;

  /** A paused World of lone Cells, two orange Carriers hiding jade and one pure jade, and the id of jade. */
  function lonelyJade(h: Harness): bigint {
    start(h, 15, false, EMPTY);
    h.send({ type: 'paint', cells: [[3, 4], [9, 9]], genotype: [JADE, ORANGE] });
    h.send({ type: 'paint', cells: [[20, 20]], genotype: [JADE, JADE] });
    h.send({ type: 'end-stroke' });
    h.send({ type: 'watch-census', watching: true });
    const census = h.notices.filter((notice): notice is Census => notice.type === 'census').at(-1);
    const jade = census?.alleles.find((line) => line.hue === JADE.hue)?.id;
    if (jade === undefined) throw new Error('jade is not in the census');
    return jade;
  }

  it('marks, in each frame, the Cells that show the highlighted Allele and the Carriers that hide it', () => {
    const h = harness();
    const jade = lonelyJade(h);
    h.send({ type: 'highlight', allele: jade });
    h.clock.advance(0);
    const highlight = h.latest().highlight;
    expect(highlight).toMatchObject({ allele: jade, shown: 1, hidden: 2 });
    const marked = [...(highlight?.carriers ?? [])].flatMap((mark, index) => (mark === 0 ? [] : [[index, mark]]));
    expect(marked).toEqual([
      [at(3, 4), 2],
      [at(9, 9), 2],
      [at(20, 20), 1],
    ]);
  });

  it('follows the run', () => {
    const h = harness();
    const jade = lonelyJade(h);
    h.send({ type: 'highlight', allele: jade });
    h.send({ type: 'step' });
    h.clock.advance(0);
    const highlight = h.latest().highlight;
    expect(highlight).toMatchObject({ allele: jade, shown: 0, hidden: 0 });
    expect(highlight?.carriers.every((mark) => mark === 0)).toBe(true);
  });

  it('keeps marking while the page holds frames back and hands them back', () => {
    const h = harness({ keepUp: false });
    const jade = lonelyJade(h);
    h.send({ type: 'highlight', allele: jade });
    for (let i = 0; i < 5; i++) {
      h.releaseAll();
      h.send({ type: 'paint', cells: [[i, 30]], genotype: [JADE, JADE] });
    }
    h.releaseAll();
    expect(h.errors).toEqual([]);
    expect(h.latest().highlight).toMatchObject({ shown: 6, hidden: 2 });
  });

  it('ends when cleared, and in a new Soup', () => {
    const h = harness();
    const jade = lonelyJade(h);
    h.send({ type: 'highlight', allele: jade });
    h.send({ type: 'highlight', allele: null });
    h.clock.advance(0);
    expect(h.latest().highlight).toBeNull();

    h.send({ type: 'highlight', allele: jade });
    start(h, 15, false, EMPTY);
    expect(h.latest().highlight).toBeNull();
  });
});

describe('census', () => {
  const EMPTY: SoupSettings = { ...SOUP, width: 32, height: 32, density: 0, mutation: { rate: 0, magnitude: 0 } };
  const ORANGE = { hue: 40, dominance: 2 };
  const JADE = { hue: 160, dominance: 1 };
  const censuses = (h: Harness): Census[] => h.notices.filter((notice): notice is Census => notice.type === 'census');
  const latestCensus = (h: Harness): Census => {
    const census = censuses(h).at(-1);
    if (!census) throw new Error('no census yet');
    return census;
  };

  it('is not sent until the page watches it', () => {
    const h = harness();
    start(h);
    h.clock.advance(1000);
    expect(censuses(h)).toEqual([]);
  });

  it('reports the Generation, Population, and how many Organisms show or hide each living Allele', () => {
    const h = harness();
    start(h, 15, false, EMPTY);
    h.send({ type: 'step' });
    h.send({ type: 'paint', cells: [[3, 4], [9, 9]], genotype: [JADE, ORANGE] });
    h.send({ type: 'paint', cells: [[20, 20]], genotype: [JADE, JADE] });
    h.send({ type: 'watch-census', watching: true });
    expect(latestCensus(h)).toEqual({
      type: 'census',
      generation: 1,
      population: 3,
      alleles: [
        { ...JADE, id: expect.any(BigInt), copies: 4, shown: 1, hidden: 2 },
        { ...ORANGE, id: expect.any(BigInt), copies: 2, shown: 2, hidden: 0 },
      ],
      tree: expect.any(Array),
      populationHistory: Uint32Array.of(0, 3),
    });
  });

  it('carries the Allele Tree: each Allele with its source, origin, when it was last carried and its status', () => {
    const h = harness();
    start(h, 15, false, EMPTY);
    h.send({ type: 'step' });
    h.send({ type: 'paint', cells: [[3, 4]], genotype: [JADE, ORANGE] });
    h.send({ type: 'end-stroke' });
    h.send({ type: 'step' });
    h.send({ type: 'step' });
    h.send({ type: 'paint', cells: [[9, 9]], genotype: [ORANGE, ORANGE] });
    h.send({ type: 'watch-census', watching: true });
    const { tree, alleles } = latestCensus(h);
    expect(tree).toHaveLength(SOUP.foundingAlleles + 2);
    const founding = tree.slice(0, SOUP.foundingAlleles);
    expect(founding.every((node) => node.source === null && node.origin === 0 && node.lastCarried === 0 && node.status === 'extinct')).toBe(true);
    expect(tree.slice(SOUP.foundingAlleles)).toEqual([
      { ...JADE, id: expect.any(BigInt), source: null, origin: 1, lastCarried: 1, status: 'extinct' },
      { ...ORANGE, id: alleles[0]?.id, source: null, origin: 1, lastCarried: 3, status: 'living' },
    ]);
  });

  it('links each mutant in the Allele Tree to the Allele it mutated from', () => {
    const h = harness();
    start(h, 15, false, { ...SOUP, width: 48, height: 48, mutation: { rate: 0.2, magnitude: 30 } });
    for (let i = 0; i < 20; i++) h.send({ type: 'step' });
    h.send({ type: 'watch-census', watching: true });
    const { tree, alleles, generation } = latestCensus(h);
    const seen = new Set<bigint>();
    for (const node of tree) {
      if (node.source !== null) expect(seen).toContain(node.source);
      expect(node.lastCarried).toBeGreaterThanOrEqual(node.origin);
      expect(node.lastCarried).toBeLessThanOrEqual(generation);
      seen.add(node.id);
    }
    expect(tree.some((node) => node.source !== null)).toBe(true);
    const living = tree.filter((node) => node.status !== 'extinct');
    expect(living.map((node) => node.id).sort()).toEqual(alleles.map((line) => line.id).sort());
    expect(living.every((node) => node.lastCarried === generation)).toBe(true);
  });

  it('follows the run at most four times a second, and always ends on the latest Generation', () => {
    const h = harness();
    h.cost.msPerGeneration = 1;
    start(h, 'max', true);
    h.send({ type: 'watch-census', watching: true });
    h.clock.advance(1000);
    h.send({ type: 'pause' });
    h.clock.advance(1000);
    const sent = censuses(h);
    expect(sent.length).toBeGreaterThan(2);
    expect(sent.length).toBeLessThanOrEqual(6);
    const last = latestCensus(h);
    expect(last.generation).toBe(h.latest().generation);
    expect(last.population).toBe(h.latest().population);
    expect(last.populationHistory).toHaveLength(last.generation + 1);
    expect(last.populationHistory.at(-1)).toBe(last.population);
  });

  it('stops when the page stops watching', () => {
    const h = harness();
    start(h);
    h.send({ type: 'watch-census', watching: true });
    h.clock.advance(1000);
    h.send({ type: 'watch-census', watching: false });
    const count = censuses(h).length;
    h.clock.advance(2000);
    expect(censuses(h)).toHaveLength(count);
  });

  it('reports a new Soup straight away', () => {
    const h = harness();
    start(h, 15, false, EMPTY);
    h.send({ type: 'watch-census', watching: true });
    h.send({ type: 'paint', cells: [[1, 1]], genotype: [ORANGE, ORANGE] });
    start(h, 15, false, { ...EMPTY, width: 16, height: 16 });
    expect(latestCensus(h)).toMatchObject({ generation: 0, population: 0, alleles: [] });
  });
});

describe('Share Links', () => {
  /** The core's Share Link of {@link SOUP}. */
  const SOUP_LINK = 'v=1&seed=42&rule=B3/S23&width=160&height=160&density=0.3&alleles=8&mutation-rate=0.01&mutation-magnitude=30';

  it('are sent for a new Soup', () => {
    const h = harness();
    start(h, 15, false);
    expect(links(h)).toEqual([SOUP_LINK]);
  });

  it('start exactly the run of the Soup they describe', () => {
    const h = harness();
    h.send({ type: 'open-link', link: `https://example.com/app/#${SOUP_LINK}`, rate: 15, running: true });
    h.clock.advance(2000);
    const frame = h.latest();
    expect(frame.generation).toBe(30);
    expect(frame.phenotypes).toEqual(referencePhenotypes(SOUP, 30));
    expect(h.persistence).toContainEqual({ type: 'world', source: 'link', rule: 'B3/S23', mutation: { rate: 0.01, magnitude: 30 } });
    expect(links(h)).toEqual([SOUP_LINK]);
  });

  it('carry the Rule and Mutation settings into the run', () => {
    const h = harness();
    const link = 'v=1&seed=7&rule=B36/S23&width=64&height=48&density=0.5&alleles=3&mutation-rate=0.2&mutation-magnitude=90';
    h.send({ type: 'open-link', link, rate: 15, running: false });
    expect(h.persistence[0]).toEqual({ type: 'world', source: 'link', rule: 'B36/S23', mutation: { rate: 0.2, magnitude: 90 } });
    h.send({ type: 'step' });
    const soup: SoupSettings = { seed: 7n, rule: 'B36/S23', width: 64, height: 48, density: 0.5, foundingAlleles: 3, mutation: { rate: 0.2, magnitude: 90 } };
    expect(h.latest().phenotypes).toEqual(referencePhenotypes(soup, 1));
  });

  it('that the core cannot decode are rejected with its explanation, leaving the World running as it was', () => {
    const h = harness();
    start(h);
    h.clock.advance(1000);
    h.send({ type: 'open-link', link: SOUP_LINK.replace('v=1', 'v=9'), rate: 15, running: true });
    h.send({ type: 'open-link', link: SOUP_LINK.replace('width=160', 'width=8'), rate: 15, running: true });
    h.clock.advance(1000);
    expect(h.persistence.filter((event) => event.type === 'rejected')).toEqual([
      { type: 'rejected', source: 'link', message: 'share link version \'9\' is not supported, this build reads version 1' },
      { type: 'rejected', source: 'link', message: expect.stringMatching(/^share link world settings are invalid: .*8x160/) },
    ]);
    expect(h.latest().generation).toBe(30);
    expect(h.latest().phenotypes).toEqual(referencePhenotypes(SOUP, 30));
    expect(links(h)).toEqual([SOUP_LINK]);
  });

  it('stop describing the World once it is edited', () => {
    const h = harness();
    start(h, 15, false);
    h.send({ type: 'paint', cells: [[1, 1]], genotype: [{ hue: 40, dominance: 2 }, { hue: 40, dominance: 2 }] });
    h.send({ type: 'end-stroke' });
    h.send({ type: 'undo' });
    expect(links(h)).toEqual([SOUP_LINK, null]);
  });

  it('follow a Rule or Mutation change at Generation 0', () => {
    const h = harness();
    start(h, 15, false);
    h.send({ type: 'set-rule', rule: 'b63/s32' });
    h.send({ type: 'set-mutation', mutation: { rate: 0.05, magnitude: 30 } });
    expect(links(h)).toEqual([
      SOUP_LINK,
      SOUP_LINK.replace('B3/S23', 'B36/S23'),
      SOUP_LINK.replace('B3/S23', 'B36/S23').replace('mutation-rate=0.01', 'mutation-rate=0.05'),
    ]);
  });

  it('stop describing the run once its Rule or Mutation changes after Generation 0, but not for an unchanged setting', () => {
    const rule = harness();
    start(rule, 15, false);
    rule.send({ type: 'step' });
    rule.send({ type: 'set-rule', rule: 'B3/S23' });
    rule.send({ type: 'set-mutation', mutation: { rate: 0.01, magnitude: 30 } });
    expect(links(rule)).toEqual([SOUP_LINK]);
    rule.send({ type: 'set-rule', rule: 'B36/S23' });
    expect(links(rule)).toEqual([SOUP_LINK, null]);

    const mutation = harness();
    start(mutation, 15, false);
    mutation.send({ type: 'step' });
    mutation.send({ type: 'set-mutation', mutation: { rate: 0.01, magnitude: 45 } });
    expect(links(mutation)).toEqual([SOUP_LINK, null]);
  });
});

describe('Snapshots', () => {
  const snapshots = (h: Harness): Extract<Persistence, { type: 'snapshot' }>[] =>
    h.persistence.flatMap((event) => (event.type === 'snapshot' ? [event] : []));

  function saved(h: Harness, purpose: 'export' | 'autosave' = 'export'): Extract<Persistence, { type: 'snapshot' }> {
    h.send({ type: 'save', purpose });
    const snapshot = snapshots(h).at(-1);
    if (!snapshot) throw new Error('no snapshot sent');
    return snapshot;
  }

  it('resume the run exactly where it was saved', () => {
    const h = harness();
    start(h);
    h.clock.advance(1000);
    h.send({ type: 'pause' });
    const snapshot = saved(h);
    expect(snapshot).toMatchObject({ purpose: 'export', generation: 15 });

    const resumed = harness();
    resumed.send({ type: 'import', snapshot: snapshot.snapshot, link: null, source: 'snapshot', rate: 15, running: true });
    expect(resumed.latest().generation).toBe(15);
    resumed.clock.advance(1000);
    expect(resumed.latest().generation).toBe(30);
    expect(resumed.latest().phenotypes).toEqual(referencePhenotypes(SOUP, 30));
  });

  it('bring back the Rule and Mutation settings of the run', () => {
    const h = harness();
    start(h, 15, false, { ...SOUP, rule: LIFE_WITHOUT_DEATH, mutation: { rate: 0.2, magnitude: 90 } });
    const snapshot = saved(h).snapshot;
    const resumed = harness();
    resumed.send({ type: 'import', snapshot, link: null, source: 'snapshot', rate: 15, running: false });
    expect(resumed.persistence[0]).toEqual({ type: 'world', source: 'snapshot', rule: LIFE_WITHOUT_DEATH, mutation: { rate: 0.2, magnitude: 90 } });
  });

  it('resume an edited World, which no Share Link describes', () => {
    const h = harness();
    start(h, 15, false, { ...SOUP, width: 32, height: 32, density: 0 });
    h.send({ type: 'paint', cells: [[3, 4], [4, 4], [5, 4]], genotype: [{ hue: 40, dominance: 2 }, { hue: 230, dominance: 0 }] });
    const snapshot = saved(h);
    expect(snapshot.link).toBeNull();
    const resumed = harness();
    resumed.send({ type: 'import', snapshot: snapshot.snapshot, link: null, source: 'snapshot', rate: 15, running: false });
    resumed.send({ type: 'step' });
    expect(resumed.latest().population).toBe(3);
    expect(resumed.latest().phenotypes.filter((hue) => hue >= 0)).toHaveLength(3);
  });

  it('carry the Share Link of the run, so an autosaved run stays shareable once restored', () => {
    const h = harness();
    start(h, 15, false);
    h.send({ type: 'step' });
    const snapshot = saved(h, 'autosave');
    expect(snapshot.purpose).toBe('autosave');
    expect(snapshot.link).toBe(links(h)[0]);

    const restored = harness();
    restored.send({ type: 'import', snapshot: snapshot.snapshot, link: snapshot.link, source: 'autosave', rate: 15, running: false });
    expect(links(restored)).toEqual([snapshot.link]);
    const imported = harness();
    imported.send({ type: 'import', snapshot: snapshot.snapshot, link: null, source: 'snapshot', rate: 15, running: false });
    expect(links(imported)).toEqual([]);
  });

  it('that are corrupt or of an unknown version are rejected with the core explanation, leaving the World running as it was', () => {
    const h = harness();
    start(h);
    h.clock.advance(1000);
    const snapshot = saved(h).snapshot;
    const newer = snapshot.slice();
    newer[4] = 99;
    const corrupt = snapshot.slice(0, snapshot.length - 8);
    for (const bad of [new TextEncoder().encode('not a snapshot'), newer, corrupt]) {
      h.send({ type: 'import', snapshot: bad, link: null, source: 'snapshot', rate: 15, running: true });
    }
    h.clock.advance(1000);
    expect(h.persistence.filter((event) => event.type === 'rejected')).toEqual([
      { type: 'rejected', source: 'snapshot', message: 'not a snapshot: the data does not start with GGLS' },
      { type: 'rejected', source: 'snapshot', message: 'snapshot format version 99 is not supported, this build reads version 4' },
      { type: 'rejected', source: 'snapshot', message: expect.stringMatching(/^snapshot data is (corrupt|truncated)/) },
    ]);
    expect(h.latest().generation).toBe(30);
    expect(h.latest().phenotypes).toEqual(referencePhenotypes(SOUP, 30));
  });
});
