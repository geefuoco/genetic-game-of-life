import { describe, expect, it } from 'vitest';
import { cellsAlong } from './stroke.ts';

const world = { width: 160, height: 120 };

describe('cells along a drag', () => {
  it('is the one Cell under a pointer that has not moved', () => {
    expect(cellsAlong({ x: 3.7, y: 4.2 }, { x: 3.1, y: 4.9 }, world)).toEqual([[3, 4]]);
  });

  it('covers every Cell a straight drag crosses, in order', () => {
    expect(cellsAlong({ x: 1.5, y: 2.5 }, { x: 4.2, y: 2.5 }, world)).toEqual([
      [1, 2],
      [2, 2],
      [3, 2],
      [4, 2],
    ]);
  });

  it('leaves no gap in a fast diagonal drag', () => {
    const cells = cellsAlong({ x: 10.5, y: 10.5 }, { x: 30.5, y: 17.5 }, world);
    expect(cells[0]).toEqual([10, 10]);
    expect(cells.at(-1)).toEqual([30, 17]);
    for (let i = 1; i < cells.length; i++) {
      const [x0, y0] = cells[i - 1]!;
      const [x1, y1] = cells[i]!;
      expect(Math.max(Math.abs(x1 - x0), Math.abs(y1 - y0))).toBe(1);
    }
  });

  it('takes the short way across the edges of the torus', () => {
    expect(cellsAlong({ x: 158.5, y: 119.5 }, { x: 1.5, y: 0.5 }, world)).toEqual([
      [158, 119],
      [159, 119],
      [0, 0],
      [1, 0],
    ]);
  });
});
