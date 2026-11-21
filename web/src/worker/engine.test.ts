import { readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { initSync, WebWorld } from '../../wasm-pkg/genetic_life_wasm.js';
import type { Command, Frame, Rate, SoupSettings, WorkerEvent } from '../protocol.ts';
import { RULE_PRESETS } from '../lib/settings.ts';
import { Engine, type Clock, type EngineWorld } from './engine.ts';

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

interface Harness {
  clock: FakeClock;
  frames: Frame[];
  errors: string[];
  /** Events other than frames and errors, in order. */
  notices: Exclude<WorkerEvent, Frame | { type: 'error' }>[];
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
  const held: Float32Array[] = [];
  const cost = { msPerGeneration: 0 };
  const createWorld = (soup: SoupSettings): EngineWorld => {
    const world = createWebWorld(soup);
    return {
      step(generations) {
        world.step(generations);
        clock.time += cost.msPerGeneration * generations;
      },
      generation: () => world.generation(),
      population: () => world.population(),
      phenotypes: (out) => world.phenotypes(out),
      setRule: (rule) => world.setRule(rule),
      setMutation: (rate, magnitude) => world.setMutation(rate, magnitude),
      paint: (x, y, genotype) => world.paint(x, y, genotype),
      erase: (x, y) => world.erase(x, y),
      stamp: (pattern, x, y, genotype) => world.stamp(pattern, x, y, genotype),
      endStroke: () => world.endStroke(),
      undo: () => world.undo(),
      redo: () => world.redo(),
      canUndo: () => world.canUndo(),
      canRedo: () => world.canRedo(),
      free: () => world.free(),
    };
  };
  const onEvent = (event: WorkerEvent): void => {
    if (event.type === 'error') {
      errors.push(event.message);
      return;
    }
    if (event.type !== 'frame') {
      notices.push(event);
      return;
    }
    frames.push({ ...event, phenotypes: event.phenotypes.slice() });
    if (keepUp) queueRelease(event.phenotypes);
    else held.push(event.phenotypes);
  };
  const releases: Float32Array[] = [];
  const queueRelease = (buffer: Float32Array): void => {
    releases.push(buffer);
    clock.setTimeout(() => {
      const next = releases.shift();
      if (next) engine.handle({ type: 'release', phenotypes: next });
    }, 0);
  };
  const engine = new Engine({ createWorld, clock, send: onEvent });
  return {
    clock,
    frames,
    errors,
    notices,
    cost,
    send: (command) => engine.handle(command),
    releaseAll() {
      for (const buffer of held.splice(0)) engine.handle({ type: 'release', phenotypes: buffer });
    },
    latest() {
      const frame = frames.at(-1);
      if (!frame) throw new Error('no frame yet');
      return frame;
    },
  };
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
