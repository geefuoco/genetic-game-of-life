import { describe, expect, it } from 'vitest';
import {
  MAX_SCALE,
  minScale,
  openingCamera,
  panBy,
  resizeViewport,
  screenToCell,
  worldWindow,
  zoomAt,
} from './camera.ts';

const viewport = { width: 1440, height: 900 };
const soup = { width: 160, height: 160 };
const large = { width: 1024, height: 1024 };
const centre = { x: 720, y: 450 };

function expectSameCell(a: { x: number; y: number }, b: { x: number; y: number }): void {
  expect(a.x).toBeCloseTo(b.x, 6);
  expect(a.y).toBeCloseTo(b.y, 6);
}

describe('opening camera', () => {
  it('opens at 9 px per Cell', () => {
    expect(openingCamera(soup, viewport).scale).toBe(9);
  });

  it('opens with the middle of the World at the middle of the viewport', () => {
    expectSameCell(screenToCell(openingCamera(soup, viewport), soup, centre), { x: 80, y: 80 });
  });

  it('opens at the zoom-in limit when even that leaves room around a tiny World', () => {
    expect(openingCamera({ width: 16, height: 16 }, viewport).scale).toBe(MAX_SCALE);
  });
});

describe('zoom', () => {
  it('stops zooming in at 40 px per Cell', () => {
    expect(zoomAt(openingCamera(soup, viewport), soup, viewport, 1000, centre).scale).toBe(40);
  });

  it('zooms out until a 1024 World fits with a margin, below 1 px per Cell', () => {
    const scale = zoomAt(openingCamera(large, viewport), large, viewport, 1e-6, centre).scale;
    expect(scale).toBeLessThan(1);
    expect(scale).toBeCloseTo((900 - 2 * 48) / 1024, 9);
  });

  it('never zooms out past the whole World with a margin on both axes', () => {
    const wide = { width: 1024, height: 128 };
    const scale = minScale(wide, viewport);
    expect(scale * 1024).toBeCloseTo(1440 - 96, 6);
    expect(scale * 128).toBeLessThan(900 - 96);
  });

  it('zooms by fractional amounts', () => {
    expect(zoomAt(openingCamera(soup, viewport), soup, viewport, 1.1, centre).scale).toBeCloseTo(9.9, 9);
  });

  it('keeps the Cell under the anchor in place', () => {
    const before = openingCamera(soup, viewport);
    const anchor = { x: 300, y: 610 };
    const after = zoomAt(before, soup, viewport, 1.7, anchor);
    expectSameCell(screenToCell(after, soup, anchor), screenToCell(before, soup, anchor));
  });
});

describe('pan', () => {
  it('moves the World with the pointer', () => {
    const before = openingCamera(soup, viewport);
    const after = panBy(before, soup, 37, -12);
    expectSameCell(screenToCell(after, soup, { x: 437, y: 188 }), screenToCell(before, soup, { x: 400, y: 200 }));
  });

  it('wraps across the torus, so a whole World width of panning looks the same', () => {
    const start = openingCamera(soup, viewport);
    const once = panBy(start, soup, 5, 7);
    const far = panBy(start, soup, 5 + 160 * 9 * 1000, 7 - 160 * 9 * 3);
    expect(far.x).toBeCloseTo(once.x, 6);
    expect(far.y).toBeCloseTo(once.y, 6);
  });

  it('shows the Cells past one edge as the Cells at the other edge', () => {
    const camera = panBy(openingCamera(soup, viewport), soup, 0, 0);
    const leftOfCentre = screenToCell(camera, soup, { x: centre.x - 80 * 9 - 4.5, y: centre.y });
    expect(leftOfCentre.x).toBeCloseTo(159.5, 6);
  });
});

describe('resize', () => {
  it('keeps the Cell at the middle of the viewport', () => {
    const before = zoomAt(openingCamera(soup, viewport), soup, viewport, 1.3, { x: 100, y: 100 });
    const smaller = { width: 1100, height: 900 };
    const after = resizeViewport(before, soup, viewport, smaller);
    expectSameCell(screenToCell(after, soup, { x: 550, y: 450 }), screenToCell(before, soup, centre));
  });

  it('zooms in when the viewport grows past the furthest zoom-out', () => {
    const out = zoomAt(openingCamera(large, viewport), large, viewport, 1e-6, centre);
    const bigger = { width: 2560, height: 1440 };
    expect(resizeViewport(out, large, viewport, bigger).scale).toBeCloseTo(minScale(large, bigger), 9);
  });
});

describe('world window', () => {
  it('is centred and as large as the World where the World is smaller than the viewport', () => {
    const camera = zoomAt(openingCamera(soup, viewport), soup, viewport, 5.5 / 9, centre);
    const window = worldWindow(camera, soup, viewport);
    expect(window.x).toBeCloseTo(280, 6);
    expect(window.y).toBeCloseTo(10, 6);
    expect(window.width).toBeCloseTo(880, 6);
    expect(window.height).toBeCloseTo(880, 6);
  });

  it('fills the viewport on an axis where the World is larger', () => {
    expect(worldWindow(openingCamera(soup, viewport), soup, viewport)).toEqual({ x: 0, y: 0, width: 1440, height: 900 });
  });
});
