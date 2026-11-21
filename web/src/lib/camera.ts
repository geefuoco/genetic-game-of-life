/**
 * How the World maps onto the viewport: a zoom in CSS pixels per Cell and
 * the screen position of Cell (0, 0). The World is a torus, so the position
 * is kept within one World of the origin and every view of it is seamless.
 */

export interface Size {
  width: number;
  height: number;
}

export interface Point {
  x: number;
  y: number;
}

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** A view of the World. */
export interface Camera {
  /** CSS pixels per Cell. */
  scale: number;
  /** Viewport x of the left edge of Cell column 0, in `0..world width * scale`. */
  x: number;
  /** Viewport y of the top edge of Cell row 0, in `0..world height * scale`. */
  y: number;
}

/** The zoom the page opens at. */
export const OPENING_SCALE = 9;
/** The furthest zoom-in. */
export const MAX_SCALE = 40;
/** Space left around the whole World at the furthest zoom-out, in CSS pixels. */
export const FIT_MARGIN = 48;

/** The furthest zoom-out: the whole World fits with a margin on both axes. */
export function minScale(world: Size, viewport: Size): number {
  const fit = Math.min(
    (viewport.width - 2 * FIT_MARGIN) / world.width,
    (viewport.height - 2 * FIT_MARGIN) / world.height,
  );
  return Math.min(MAX_SCALE, Math.max(Number.MIN_VALUE, fit));
}

function clampScale(scale: number, world: Size, viewport: Size): number {
  return Math.min(MAX_SCALE, Math.max(minScale(world, viewport), scale));
}

/** `value` wrapped into `0..modulus`. */
function wrap(value: number, modulus: number): number {
  return ((value % modulus) + modulus) % modulus;
}

/** Brings the offsets back within one World, which leaves the view unchanged. */
function wrapped(scale: number, x: number, y: number, world: Size): Camera {
  return { scale, x: wrap(x, world.width * scale), y: wrap(y, world.height * scale) };
}

/** The view the page opens on: the World centred at the opening zoom. */
export function openingCamera(world: Size, viewport: Size): Camera {
  const scale = clampScale(OPENING_SCALE, world, viewport);
  return wrapped(
    scale,
    (viewport.width - world.width * scale) / 2,
    (viewport.height - world.height * scale) / 2,
    world,
  );
}

/** Multiplies the zoom by `factor`, within its limits, keeping the point under `anchor` in place. */
export function zoomAt(camera: Camera, world: Size, viewport: Size, factor: number, anchor: Point): Camera {
  const scale = clampScale(camera.scale * factor, world, viewport);
  const ratio = scale / camera.scale;
  return wrapped(
    scale,
    anchor.x - (anchor.x - camera.x) * ratio,
    anchor.y - (anchor.y - camera.y) * ratio,
    world,
  );
}

/** Moves the World by `dx`, `dy` CSS pixels, wrapping across the torus. */
export function panBy(camera: Camera, world: Size, dx: number, dy: number): Camera {
  return wrapped(camera.scale, camera.x + dx, camera.y + dy, world);
}

/** Follows a viewport resize, keeping the middle of the view in place and the zoom within its limits. */
export function resizeViewport(camera: Camera, world: Size, from: Size, to: Size): Camera {
  const moved = panBy(camera, world, (to.width - from.width) / 2, (to.height - from.height) / 2);
  return zoomAt(moved, world, to, 1, { x: to.width / 2, y: to.height / 2 });
}

/** The fractional Cell coordinates under a viewport point, wrapped into the World. */
export function screenToCell(camera: Camera, world: Size, point: Point): Point {
  return {
    x: wrap((point.x - camera.x) / camera.scale, world.width),
    y: wrap((point.y - camera.y) / camera.scale, world.height),
  };
}

/**
 * The window the World is drawn in: centred, and on each axis the size of
 * the World where it is smaller than the viewport, or the whole viewport.
 */
export function worldWindow(camera: Camera, world: Size, viewport: Size): Rect {
  const width = Math.min(viewport.width, world.width * camera.scale);
  const height = Math.min(viewport.height, world.height * camera.scale);
  return { x: (viewport.width - width) / 2, y: (viewport.height - height) / 2, width, height };
}

/**
 * Where Cell `(x, y)` is drawn in the viewport, or `null` unless all of it
 * is inside the World window.
 */
export function cellToScreen(camera: Camera, world: Size, viewport: Size, cell: Point): Rect | null {
  const window = worldWindow(camera, world, viewport);
  const left = window.x + wrap(camera.x + cell.x * camera.scale - window.x, world.width * camera.scale);
  const top = window.y + wrap(camera.y + cell.y * camera.scale - window.y, world.height * camera.scale);
  const slack = 0.001;
  if (left + camera.scale > window.x + window.width + slack || top + camera.scale > window.y + window.height + slack) return null;
  return { x: left, y: top, width: camera.scale, height: camera.scale };
}
