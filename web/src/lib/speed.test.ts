import { describe, expect, it } from 'vitest';
import { DEFAULT_SPEED, SPEEDS, cycleSpeed, nudgeSpeed } from './speed.ts';

const label = (index: number): string => SPEEDS[index]!.label;
const indexOf = (text: string): number => SPEEDS.findIndex((speed) => speed.label === text);

describe('speeds', () => {
  it('step through half, 1x, 2x, 4x and Max', () => {
    expect(SPEEDS.map((speed) => speed.label)).toEqual(['½×', '1×', '2×', '4×', 'Max']);
  });

  it('default to 1x at 15 Generations per second', () => {
    expect(label(DEFAULT_SPEED)).toBe('1×');
    expect(SPEEDS[DEFAULT_SPEED]!.rate).toBe(15);
  });

  it('scale from 1x', () => {
    expect(SPEEDS.map((speed) => speed.rate)).toEqual([7.5, 15, 30, 60, 'max']);
  });
});

describe('cycling speed with the button', () => {
  it('goes faster from 4x to Max', () => {
    expect(label(cycleSpeed(indexOf('4×'), 1))).toBe('Max');
  });

  it('wraps from Max round to half speed', () => {
    expect(label(cycleSpeed(indexOf('Max'), 1))).toBe('½×');
  });

  it('wraps from half speed round to Max when going slower', () => {
    expect(label(cycleSpeed(indexOf('½×'), -1))).toBe('Max');
  });
});

describe('nudging speed with keys', () => {
  it('stops at Max', () => {
    expect(label(nudgeSpeed(indexOf('Max'), 1))).toBe('Max');
  });

  it('stops at half speed', () => {
    expect(label(nudgeSpeed(indexOf('½×'), -1))).toBe('½×');
  });

  it('goes one step slower', () => {
    expect(label(nudgeSpeed(indexOf('2×'), -1))).toBe('1×');
  });
});
