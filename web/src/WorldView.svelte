<script lang="ts">
  import { onMount } from 'svelte';
  import type { Tool } from './lib/brush.ts';
  import {
    cellToScreen,
    openingCamera,
    panBy,
    resizeViewport,
    screenToCell,
    zoomAt,
    type Camera,
    type Point,
    type Rect,
    type Size,
  } from './lib/camera.ts';
  import { Renderer } from './lib/renderer.ts';
  import { cellsAlong } from './lib/stroke.ts';
  import type { CellPosition, Frame } from './protocol.ts';

  interface Props {
    /** What pressing and dragging on the World does. */
    tool: Tool;
    /** Called once on mount with whether WebGL2 is available. */
    onsupport: (supported: boolean) => void;
    /** Called with each frame's buffer once it has been drawn or superseded. */
    onrelease: (phenotypes: Float32Array) => void;
    /** Called with the Cells a press or drag of the paint or erase tool passes over. */
    onstroke: (cells: CellPosition[]) => void;
    /** Called with the Cell the stamp tool is pressed on. */
    onstamp: (cell: CellPosition) => void;
    /** Called when a press of an editing tool ends, closing its edit stroke. */
    onstrokeend: () => void;
    /** Called with the Cell the look tool is clicked or tapped on. */
    oninspect: (cell: CellPosition) => void;
    /** The inspected Organism's Cell, ringed, and whether it has died. */
    inspected: { x: number; y: number; dead: boolean } | null;
    /** Where the inspected Cell is on screen, or `null` while it is out of view or nothing is inspected. */
    anchor?: Rect | null;
  }

  let { tool, onsupport, onrelease, onstroke, onstamp, onstrokeend, oninspect, inspected, anchor = $bindable(null) }: Props = $props();

  /** Wheel zoom per pixel of scroll. */
  const WHEEL_ZOOM = 0.002;
  /** Pixels per line or page of scroll, for wheels that report in those units. */
  const LINE_PX = 16;
  /** Furthest a look press can move, in CSS pixels, and still count as a click that inspects. */
  const CLICK_SLOP = 4;

  let canvas: HTMLCanvasElement;
  let renderer: Renderer | null = null;
  let viewport: Size = { width: 0, height: 0 };
  let world: Size | null = null;
  let camera: Camera | null = null;
  let pending: Frame | null = null;
  let dirty = false;
  let drag: { x: number; y: number } | null = null;
  /** Where the look tool was pressed, until the press moves too far to be a click. */
  let pressedAt: { x: number; y: number } | null = null;
  /** Where the paint or erase tool last was during a press, in Cell coordinates. */
  let brushAt: Point | null = null;
  let editing = false;
  let grabbing = $state(false);

  /** Shows `frame` from the next animation frame on. */
  export function present(frame: Frame): void {
    if (pending) onrelease(pending.phenotypes);
    pending = frame;
    dirty = true;
  }

  /** Zooms about the middle of the viewport. */
  export function zoom(factor: number): void {
    zoomAbout(factor, { x: viewport.width / 2, y: viewport.height / 2 });
  }

  function zoomAbout(factor: number, anchor: { x: number; y: number }): void {
    if (!camera || !world) return;
    camera = zoomAt(camera, world, viewport, factor, anchor);
    dirty = true;
  }

  function draw(): void {
    if (!renderer) return;
    const frame = pending;
    if (frame) {
      pending = null;
      if (!world || world.width !== frame.width || world.height !== frame.height) {
        world = { width: frame.width, height: frame.height };
        camera = null;
      }
      renderer.upload(frame);
      onrelease(frame.phenotypes);
    }
    if (!world || viewport.width === 0 || viewport.height === 0) return;
    camera ??= openingCamera(world, viewport);
    renderer.draw({ camera, viewport, pixelRatio: window.devicePixelRatio || 1 });
    anchor = inspected ? cellToScreen(camera, world, viewport, inspected) : null;
  }

  /** Redraws whenever the inspected Cell changes, so the ring and anchor follow it. */
  $effect(() => {
    void inspected;
    dirty = true;
  });

  /** The fractional Cell coordinates under a pointer, or `null` before the first frame. */
  function cellAt(event: PointerEvent): Point | null {
    if (!camera || !world) return null;
    const bounds = canvas.getBoundingClientRect();
    return screenToCell(camera, world, { x: event.clientX - bounds.left, y: event.clientY - bounds.top });
  }

  function onpointerdown(event: PointerEvent): void {
    if (event.button !== 0) return;
    canvas.setPointerCapture(event.pointerId);
    if (tool === 'look') {
      drag = { x: event.clientX, y: event.clientY };
      pressedAt = drag;
      grabbing = true;
      return;
    }
    const at = cellAt(event);
    if (!at || !world) return;
    editing = true;
    if (tool === 'stamp') {
      onstamp([Math.floor(at.x), Math.floor(at.y)]);
      return;
    }
    brushAt = at;
    onstroke(cellsAlong(at, at, world));
  }

  function onpointermove(event: PointerEvent): void {
    if (brushAt && world) {
      const at = cellAt(event);
      if (!at) return;
      const cells = cellsAlong(brushAt, at, world).slice(1);
      brushAt = at;
      if (cells.length > 0) onstroke(cells);
      return;
    }
    if (!drag || !camera || !world) return;
    if (pressedAt && Math.hypot(event.clientX - pressedAt.x, event.clientY - pressedAt.y) > CLICK_SLOP) pressedAt = null;
    camera = panBy(camera, world, event.clientX - drag.x, event.clientY - drag.y);
    drag = { x: event.clientX, y: event.clientY };
    dirty = true;
  }

  function onpointerup(event: PointerEvent): void {
    if (pressedAt) {
      const at = cellAt(event);
      if (at) oninspect([Math.floor(at.x), Math.floor(at.y)]);
    }
    endDrag();
  }

  function endDrag(): void {
    drag = null;
    pressedAt = null;
    grabbing = false;
    brushAt = null;
    if (editing) {
      editing = false;
      onstrokeend();
    }
  }

  function onwheel(event: WheelEvent): void {
    event.preventDefault();
    const unit = event.deltaMode === WheelEvent.DOM_DELTA_LINE ? LINE_PX : event.deltaMode === WheelEvent.DOM_DELTA_PAGE ? viewport.height : 1;
    const bounds = canvas.getBoundingClientRect();
    zoomAbout(Math.exp(-event.deltaY * unit * WHEEL_ZOOM), { x: event.clientX - bounds.left, y: event.clientY - bounds.top });
  }

  onMount(() => {
    renderer = Renderer.create(canvas);
    onsupport(renderer !== null);
    if (!renderer) return;
    const resize = new ResizeObserver(([entry]) => {
      if (!entry) return;
      const next = { width: entry.contentRect.width, height: entry.contentRect.height };
      if (camera && world && viewport.width > 0 && viewport.height > 0) camera = resizeViewport(camera, world, viewport, next);
      viewport = next;
      dirty = true;
    });
    resize.observe(canvas);
    canvas.addEventListener('wheel', onwheel, { passive: false });
    let request = requestAnimationFrame(function loop() {
      if (dirty) {
        dirty = false;
        draw();
      }
      request = requestAnimationFrame(loop);
    });
    return () => {
      cancelAnimationFrame(request);
      resize.disconnect();
      canvas.removeEventListener('wheel', onwheel);
    };
  });
</script>

<canvas
  bind:this={canvas}
  class:grabbing
  class:editing={tool !== 'look'}
  aria-label="The World"
  {onpointerdown}
  {onpointermove}
  {onpointerup}
  onpointercancel={endDrag}
></canvas>
{#if anchor}
  {@const pad = Math.max(3, Math.round(anchor.width * 0.3))}
  <div
    class="ring"
    class:dead={inspected?.dead}
    style:left="{anchor.x - pad}px"
    style:top="{anchor.y - pad}px"
    style:width="{anchor.width + pad * 2}px"
    style:height="{anchor.height + pad * 2}px"
    style:border-radius="{anchor.width * 0.34 + pad}px"
  ></div>
{/if}

<style>
  canvas {
    position: absolute;
    inset: 0;
    display: block;
    width: 100%;
    height: 100%;
    cursor: grab;
    touch-action: none;
  }

  canvas.grabbing {
    cursor: grabbing;
  }

  canvas.editing {
    cursor: crosshair;
  }

  .ring {
    position: absolute;
    pointer-events: none;
    border: 2px solid #fff;
    box-shadow:
      0 0 0 1px rgb(0 0 0 / 60%),
      inset 0 0 0 1px rgb(0 0 0 / 60%);
  }

  .ring.dead {
    border-color: rgb(255 255 255 / 45%);
  }
</style>
