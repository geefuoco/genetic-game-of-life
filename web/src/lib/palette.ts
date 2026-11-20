/**
 * The "Natural lightness" palette: maps a Phenotype hue to an sRGB colour via
 * OKLCH, with lightness following the hue as in natural colours (yellow
 * lightest, blue and violet darkest) and chroma near the sRGB gamut edge.
 */

/** An OKLCH colour: lightness `0..1`, chroma, and hue in degrees. */
export interface Oklch {
  l: number;
  c: number;
  h: number;
}

/** Lightness is `BASE + SWING * cos(hue - LIGHTEST_HUE)`. */
const LIGHTNESS_BASE = 0.75;
const LIGHTNESS_SWING = 0.11;
const LIGHTEST_HUE = 105;
/** Fraction of the most saturated in-gamut chroma that is used. */
const GAMUT_FRACTION = 0.92;
/** Highest chroma used, whatever the gamut allows. */
const CHROMA_CAP = 0.25;

const radians = (degrees: number): number => (degrees * Math.PI) / 180;

/** Linear-light sRGB of an OKLCH colour, unclamped. */
function oklchToLinearSrgb({ l, c, h }: Oklch): [number, number, number] {
  const a = c * Math.cos(radians(h));
  const b = c * Math.sin(radians(h));
  const lc = (l + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const mc = (l - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const sc = (l - 0.0894841775 * a - 1.291485548 * b) ** 3;
  return [
    4.0767416621 * lc - 3.3077115913 * mc + 0.2309699292 * sc,
    -1.2684380046 * lc + 2.6097574011 * mc - 0.3413193965 * sc,
    -0.0041960863 * lc - 0.7034186147 * mc + 1.707614701 * sc,
  ];
}

/** The sRGB transfer function, extended to negative values by symmetry. */
function encode(linear: number): number {
  const magnitude = Math.abs(linear);
  const encoded = magnitude <= 0.0031308 ? 12.92 * magnitude : 1.055 * magnitude ** (1 / 2.4) - 0.055;
  return Math.sign(linear) * encoded;
}

/** Gamma-encoded sRGB channels of an OKLCH colour, unclamped, so out-of-gamut colours fall outside `0..1`. */
export function oklchToSrgb(colour: Oklch): [number, number, number] {
  const [r, g, b] = oklchToLinearSrgb(colour);
  return [encode(r), encode(g), encode(b)];
}

function inGamut(colour: Oklch): boolean {
  return oklchToLinearSrgb(colour).every((v) => v >= 0 && v <= 1);
}

/** The largest chroma that sRGB can show at lightness `l` and hue `h`. */
export function maxChroma(l: number, h: number): number {
  let low = 0;
  let high = 0.5;
  for (let i = 0; i < 24; i++) {
    const mid = (low + high) / 2;
    if (inGamut({ l, c: mid, h })) low = mid;
    else high = mid;
  }
  return low;
}

/** The palette's OKLCH colour for a Phenotype hue in degrees. */
export function paletteColour(hue: number): Oklch {
  const l = LIGHTNESS_BASE + LIGHTNESS_SWING * Math.cos(radians(hue - LIGHTEST_HUE));
  const c = Math.min(CHROMA_CAP, GAMUT_FRACTION * maxChroma(l, hue));
  return { l, c, h: hue };
}

/**
 * The palette sampled at `size` evenly spaced hues from 0°, as RGBA8 texels
 * for the renderer's colour lookup.
 */
export function paletteTable(size: number): Uint8Array {
  const table = new Uint8Array(size * 4);
  for (let i = 0; i < size; i++) {
    const rgb = oklchToSrgb(paletteColour((i * 360) / size));
    rgb.forEach((v, channel) => {
      table[i * 4 + channel] = Math.round(Math.min(1, Math.max(0, v)) * 255);
    });
    table[i * 4 + 3] = 255;
  }
  return table;
}
