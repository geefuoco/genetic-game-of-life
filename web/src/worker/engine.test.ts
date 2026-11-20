import { readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { initSync, WebWorld } from '../../wasm-pkg/genetic_life_wasm.js';
import type { Command, Frame, Rate, SoupSettings, WorkerEvent } from '../protocol.ts';
import { Engine, type Clock, type EngineWorld } from './engine.ts';

beforeAll(() => {
  initSync({ module: readFileSync(new URL('../../wasm-pkg/genetic_life_wasm_bg.wasm', import.meta.url)) });
});

const SOUP: SoupSettings = { seed: 42n, width: 160, height: 160, density: 0.3, foundingAlleles: 8 };

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
  const held: Float32Array[] = [];
  const cost = { msPerGeneration: 0 };
  const createWorld = (soup: SoupSettings): EngineWorld => {
    const world = new WebWorld(soup.seed, soup.width, soup.height, soup.density, soup.foundingAlleles);
    return {
      step(generations) {
        world.step(generations);
        clock.time += cost.msPerGeneration * generations;
      },
      generation: () => world.generation(),
      population: () => world.population(),
      phenotypes: (out) => world.phenotypes(out),
      free: () => world.free(),
    };
  };
  const onEvent = (event: WorkerEvent): void => {
    if (event.type === 'error') {
      errors.push(event.message);
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
  const world = new WebWorld(soup.seed, soup.width, soup.height, soup.density, soup.foundingAlleles);
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
