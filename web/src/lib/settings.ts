/** Choices and labels for the run settings: Rule, Mutation and new Soup. */

import type { SoupSettings } from '../protocol.ts';

/** A Rule Preset as the settings offer it. */
export interface RulePreset {
  label: string;
  /** The Rule in canonical B/S notation, as the core writes it. */
  notation: string;
}

/** The Rule Presets the core knows, Conway's Life first. */
export const RULE_PRESETS: readonly RulePreset[] = [
  { label: "Conway's Life", notation: 'B3/S23' },
  { label: 'HighLife', notation: 'B36/S23' },
  { label: 'Day & Night', notation: 'B3678/S34678' },
  { label: 'Seeds', notation: 'B2/S' },
  { label: 'Gnarl', notation: 'B1/S1' },
  { label: 'Life without Death', notation: 'B3/S012345678' },
];

/**
 * The running Rule and the last notation the core refused, if no Rule has
 * been confirmed since. A fresh object arrives with each answer from the
 * worker, so the custom Rule field resets even when the Rule is unchanged.
 */
export interface RuleStatus {
  running: string;
  rejected: { rule: string; message: string } | null;
}

/** World sizes offered as one-press choices for both width and height. */
export const SIZE_PRESETS: readonly number[] = [128, 256, 512, 1024];

/** The Rule Preset whose canonical notation is `notation`, or `null` for a custom Rule. */
export function presetOf(notation: string): RulePreset | null {
  return RULE_PRESETS.find((preset) => preset.notation === notation) ?? null;
}

/** A Mutation rate as the settings show it, such as "1.0%". */
export function mutationRateLabel(rate: number): string {
  return `${(rate * 100).toFixed(1)}%`;
}

/** A Mutation magnitude as the settings show it, such as "±30°". */
export function mutationReachLabel(magnitude: number): string {
  return `±${magnitude}°`;
}

/** A Soup density as the settings show it, such as "30%". */
export function densityLabel(density: number): string {
  return `${Math.round(density * 100)}%`;
}

/** What the New Soup form holds; a number field is `null` while it holds no number. */
export interface SoupDraft {
  foundingAlleles: number;
  density: number;
  width: number | null;
  height: number | null;
  /** Digits of the Seed, or empty for a random one. */
  seed: string;
}

/** The parts of a Soup the New Soup form sets; the Rule and Mutation settings carry over from the run. */
export type NewSoup = Omit<SoupSettings, 'rule' | 'mutation'>;

/** The form fields a visitor types into, which can hold something invalid. */
export type SoupField = 'width' | 'height' | 'seed';

/** Smallest and largest World width and height, as the core allows. */
const MIN_SIZE = 16;
const MAX_SIZE = 1024;
/** Largest Seed: the core's Seed is an unsigned 64-bit number. */
const MAX_SEED = 2n ** 64n - 1n;

export type SoupDraftResult = { ok: true; soup: NewSoup } | { ok: false; problems: Partial<Record<SoupField, string>> };

/** Reads the New Soup form, drawing a Seed from `randomSeed` when none is given. */
export function readSoupDraft(draft: SoupDraft, randomSeed: () => bigint): SoupDraftResult {
  const problems: Partial<Record<SoupField, string>> = {};
  const width = readSize(draft.width);
  const height = readSize(draft.height);
  if (width === null) problems.width = `Width must be a whole number from ${MIN_SIZE} to ${MAX_SIZE}.`;
  if (height === null) problems.height = `Height must be a whole number from ${MIN_SIZE} to ${MAX_SIZE}.`;
  const seed = readSeed(draft.seed, randomSeed);
  if (seed === null) problems.seed = `Seed must be a whole number from 0 to ${MAX_SEED}, or empty for a random one.`;
  if (width === null || height === null || seed === null) return { ok: false, problems };
  return {
    ok: true,
    soup: {
      seed,
      width,
      height,
      density: draft.density,
      foundingAlleles: draft.foundingAlleles,
    },
  };
}

function readSeed(text: string, randomSeed: () => bigint): bigint | null {
  const digits = text.trim();
  if (digits === '') return randomSeed();
  if (!/^\d+$/.test(digits)) return null;
  const seed = BigInt(digits);
  return seed <= MAX_SEED ? seed : null;
}

function readSize(size: number | null): number | null {
  return size !== null && Number.isInteger(size) && size >= MIN_SIZE && size <= MAX_SIZE ? size : null;
}
