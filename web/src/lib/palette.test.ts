import { describe, expect, it } from 'vitest';
import { maxChroma, oklchToSrgb, paletteColour } from './palette.ts';

const SRGB_RED = { l: 0.62796, c: 0.25768, h: 29.2339 };

describe('OKLCH to sRGB', () => {
  it('converts the OKLCH coordinates of sRGB red back to red', () => {
    const [r, g, b] = oklchToSrgb(SRGB_RED);
    expect(r).toBeCloseTo(1, 3);
    expect(g).toBeCloseTo(0, 3);
    expect(b).toBeCloseTo(0, 3);
  });

  it('converts full lightness with no chroma to white', () => {
    expect(oklchToSrgb({ l: 1, c: 0, h: 200 }).map((v) => v.toFixed(4))).toEqual(['1.0000', '1.0000', '1.0000']);
  });
});

describe('maxChroma', () => {
  it('finds sRGB red on the gamut edge at its lightness and hue', () => {
    expect(maxChroma(SRGB_RED.l, SRGB_RED.h)).toBeCloseTo(SRGB_RED.c, 3);
  });
});

describe('Natural lightness palette', () => {
  const lightnesses = Array.from({ length: 360 }, (_, hue) => ({ hue, l: paletteColour(hue).l }));

  it('makes yellow the lightest hue', () => {
    const lightest = lightnesses.reduce((a, b) => (b.l > a.l ? b : a));
    expect(lightest.hue).toBe(105);
    expect(lightest.l).toBeCloseTo(0.86, 6);
  });

  it('makes blue-violet the darkest hue', () => {
    const darkest = lightnesses.reduce((a, b) => (b.l < a.l ? b : a));
    expect(darkest.hue).toBe(285);
    expect(darkest.l).toBeCloseTo(0.64, 6);
  });

  it('caps chroma at 0.25 where the gamut allows more', () => {
    expect(paletteColour(328).c).toBe(0.25);
  });

  it('keeps chroma just inside the gamut edge where the cap is not reached', () => {
    const yellow = paletteColour(105);
    expect(yellow.c).toBeLessThan(0.25);
    const edge = oklchToSrgb({ ...yellow, c: yellow.c / 0.92 });
    const nearEdge = edge.some((v) => Math.abs(v) < 2e-3 || Math.abs(v - 1) < 2e-3);
    expect(nearEdge).toBe(true);
    expect(oklchToSrgb(yellow).every((v) => v > 2e-3 && v < 1 - 2e-3)).toBe(true);
  });

  it('gives an in-gamut sRGB colour for every hue', () => {
    for (let hue = 0; hue < 360; hue += 0.5) {
      for (const v of oklchToSrgb(paletteColour(hue))) {
        expect(v).toBeGreaterThanOrEqual(0);
        expect(v).toBeLessThanOrEqual(1);
      }
    }
  });
});
