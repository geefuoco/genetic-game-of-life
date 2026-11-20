<script lang="ts">
  import { onMount } from 'svelte';
  import { openingCamera, panBy, resizeViewport, zoomAt, type Camera, type Size } from './lib/camera.ts';
  import { Renderer } from './lib/renderer.ts';
  import type { Frame } from './protocol.ts';

  interface Props {
    /** Called once on mount with whether WebGL2 is available. */
    onsupport: (supported: boolean) => void;
    /** Called with each frame's buffer once it has been drawn or superseded. */
    onrelease: (phenotypes: Float32Array) => void;
  }

  let { onsupport, onrelease }: Props = $props();

  /** Wheel zoom per pixel of scroll. */
  const WHEEL_ZOOM = 0.002;
  /** Pixels per line or page of scroll, for wheels that report in those units. */
  const LINE_PX = 16;

  let canvas: HTMLCanvasElement;
  let renderer: Renderer | null = null;
  let viewport: Size = { width: 0, height: 0 };
  let world: Size | null = null;
  let camera: Camera | null = null;
  let pending: Frame | null = null;
  let dirty = false;
  let drag: { x: number; y: number } | null = null;
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
  }

  function onpointerdown(event: PointerEvent): void {
    if (event.button !== 0) return;
    canvas.setPointerCapture(event.pointerId);
    drag = { x: event.clientX, y: event.clientY };
    grabbing = true;
  }

  function onpointermove(event: PointerEvent): void {
    if (!drag || !camera || !world) return;
    camera = panBy(camera, world, event.clientX - drag.x, event.clientY - drag.y);
    drag = { x: event.clientX, y: event.clientY };
    dirty = true;
  }

  function endDrag(): void {
    drag = null;
    grabbing = false;
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
  aria-label="The World"
  {onpointerdown}
  {onpointermove}
  onpointerup={endDrag}
  onpointercancel={endDrag}
></canvas>

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
</style>
