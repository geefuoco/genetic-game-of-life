/** Which Cells a drag of an editing tool passes over. */

import type { Point, Size } from './camera.ts';
import type { CellPosition } from '../protocol.ts';

/** `value` wrapped into `0..modulus`. */
function wrap(value: number, modulus: number): number {
  return ((value % modulus) + modulus) % modulus;
}

/** The shortest signed step from `from` to `to` around a loop of `modulus`. */
function shortStep(from: number, to: number, modulus: number): number {
  const step = wrap(to - from, modulus);
  return step > modulus / 2 ? step - modulus : step;
}

/**
 * The Cells on the line between two pointer positions, given as fractional
 * Cell coordinates within the World, from the first to the last with no
 * gaps between neighbours. The line takes the short way across the World's
 * edges, as a pointer moving a little between events always does.
 */
export function cellsAlong(from: Point, to: Point, world: Size): CellPosition[] {
  const x0 = Math.floor(from.x);
  const y0 = Math.floor(from.y);
  const dx = shortStep(x0, Math.floor(to.x), world.width);
  const dy = shortStep(y0, Math.floor(to.y), world.height);
  const steps = { x: Math.abs(dx), y: Math.abs(dy) };
  const sign = { x: Math.sign(dx), y: Math.sign(dy) };
  const cells: CellPosition[] = [];
  let x = 0;
  let y = 0;
  let error = steps.x - steps.y;
  for (;;) {
    cells.push([wrap(x0 + x, world.width), wrap(y0 + y, world.height)]);
    if (x === dx && y === dy) return cells;
    const twice = 2 * error;
    if (twice > -steps.y) {
      error -= steps.y;
      x += sign.x;
    }
    if (twice < steps.x) {
      error += steps.x;
      y += sign.y;
    }
  }
}
