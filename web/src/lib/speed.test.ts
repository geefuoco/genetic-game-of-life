import { describe, expect, it } from 'vitest';
import { DEFAULT_SPEED, SPEEDS, cycleSpeed, nudgeSpeed } from './speed.ts';

const label = (index: number): string => SPEEDS[index]!.label;
const indexOf = (text: string): number => SPEEDS.findIndex((speed) => speed.label === text);

describe('speeds', () => {
  it('step through half, 1x, 2x and 4x', () => {
    expect(SPEEDS.map((speed) => speed.label)).toEqual(['½×', '1×', '2×', '4×']);
  });

  it('default to 1x at 15 Generations per second', () => {
    expect(label(DEFAULT_SPEED)).toBe('1×');
    expect(SPEEDS[DEFAULT_SPEED]!.rate).toBe(15);
  });

  it('scale from 1x', () => {
    expect(SPEEDS.map((speed) => speed.rate)).toEqual([7.5, 15, 30, 60]);
  });
});

describe('cycling speed with the button', () => {
  it('goes faster from 2x to 4x', () => {
    expect(label(cycleSpeed(indexOf('2×'), 1))).toBe('4×');
  });

  it('wraps from 4x round to half speed', () => {
    expect(label(cycleSpeed(indexOf('4×'), 1))).toBe('½×');
  });

  it('wraps from half speed round to 4x when going slower', () => {
    expect(label(cycleSpeed(indexOf('½×'), -1))).toBe('4×');
  });
});

describe('nudging speed with keys', () => {
  it('stops at 4x', () => {
    expect(label(nudgeSpeed(indexOf('4×'), 1))).toBe('4×');
  });

  it('stops at half speed', () => {
    expect(label(nudgeSpeed(indexOf('½×'), -1))).toBe('½×');
  });

  it('goes one step slower', () => {
    expect(label(nudgeSpeed(indexOf('2×'), -1))).toBe('1×');
  });
});
