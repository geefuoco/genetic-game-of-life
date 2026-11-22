import type { Rate } from '../protocol.ts';

/** One step of the speed button. */
export interface Speed {
  label: string;
  rate: Rate;
}

/** Generations per second at 1×. */
const BASE_RATE = 15;

export const SPEEDS: readonly Speed[] = [
  { label: '½×', rate: BASE_RATE / 2 },
  { label: '1×', rate: BASE_RATE },
  { label: '2×', rate: BASE_RATE * 2 },
  { label: '4×', rate: BASE_RATE * 4 },
];

/** Index into {@link SPEEDS} of the speed the page opens at. */
export const DEFAULT_SPEED = 1;

/** The next speed in `direction`, wrapping around, as the speed button does. */
export function cycleSpeed(index: number, direction: 1 | -1): number {
  return (index + direction + SPEEDS.length) % SPEEDS.length;
}

/** The next speed in `direction`, stopping at either end, as the keys do. */
export function nudgeSpeed(index: number, direction: 1 | -1): number {
  return Math.min(SPEEDS.length - 1, Math.max(0, index + direction));
}
