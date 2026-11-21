<script lang="ts">
  import { historyExtremes, reducedHistory } from './lib/census.ts';

  interface Props {
    /** The population at each Generation from 0 to the latest. */
    history: Uint32Array;
  }

  let { history }: Props = $props();

  const id = $props.id();
  /** Chart height and the plot's inset from the chart's edges, in CSS pixels. */
  const HEIGHT = 120;
  const INSET = { top: 18, right: 6, bottom: 4, left: 6 };
  /** Pixels per reduced history bucket. */
  const PIXELS_PER_BUCKET = 2;
  /** Share of the history one arrow key press moves the readout by. */
  const KEY_STEP = 0.01;

  let width = $state(0);
  /** The Generation the readout shows, or `null` for the latest. */
  let selected = $state<number | null>(null);

  const latest = $derived(Math.max(0, history.length - 1));
  const span = $derived(Math.max(1, latest));
  const extremes = $derived(historyExtremes(history));
  const peak = $derived(extremes.peak);
  const low = $derived(extremes.lowest);
  const top = $derived(Math.max(1, peak.population));
  const plotWidth = $derived(Math.max(1, width - INSET.left - INSET.right));
  const plotHeight = HEIGHT - INSET.top - INSET.bottom;
  const points = $derived(reducedHistory(history, Math.max(8, Math.round(plotWidth / PIXELS_PER_BUCKET))));
  const line = $derived(points.map((point, i) => `${i === 0 ? 'M' : 'L'}${x(point.generation)},${y(point.population)}`).join(''));
  const area = $derived(points.length === 0 ? '' : `${line}L${x(points.at(-1)?.generation ?? 0)},${y(0)}L${x(0)},${y(0)}Z`);
  const shownGeneration = $derived(selected === null ? latest : Math.min(selected, latest));
  const shownPopulation = $derived(history[shownGeneration] ?? 0);
  const label = $derived(
    `Population over time, from gen 0 to gen ${latest.toLocaleString()}: ${(history[latest] ?? 0).toLocaleString()} alive now, ` +
      `peak ${peak.population.toLocaleString()} at gen ${peak.generation.toLocaleString()}, ` +
      `lowest ${low.population.toLocaleString()} at gen ${low.generation.toLocaleString()}`,
  );

  function x(generation: number): number {
    return INSET.left + (generation / span) * plotWidth;
  }

  function y(population: number): number {
    return INSET.top + (1 - population / top) * plotHeight;
  }

  function onpointermove(event: PointerEvent): void {
    const bounds = (event.currentTarget as SVGSVGElement).getBoundingClientRect();
    const along = (event.clientX - bounds.left - INSET.left) / plotWidth;
    selected = Math.round(Math.min(1, Math.max(0, along)) * latest);
  }

  /** Moves the readout by a share of the history per arrow key, so long runs stay quick to cross. */
  function onkeydown(event: KeyboardEvent): void {
    const direction = { ArrowLeft: -1, ArrowDown: -1, ArrowRight: 1, ArrowUp: 1 }[event.key];
    if (direction === undefined) return;
    event.preventDefault();
    const step = Math.max(1, Math.round(latest * KEY_STEP));
    selected = Math.min(latest, Math.max(0, shownGeneration + direction * step));
  }
</script>

<figure class="chart" aria-labelledby="{id}-title">
  <figcaption>
    <span id="{id}-title" class="title">Population</span>
    <span class="readout" data-testid="population-readout">{shownPopulation.toLocaleString()} alive · gen {shownGeneration.toLocaleString()}</span>
  </figcaption>
  <div class="plot" bind:clientWidth={width}>
    <span class="peak" aria-hidden="true">peak {peak.population.toLocaleString()}</span>
    <svg role="img" aria-label={label} {width} height={HEIGHT} viewBox="0 0 {width} {HEIGHT}" {onpointermove} onpointerleave={() => (selected = null)}>
      <line class="grid" x1={INSET.left} x2={width - INSET.right} y1={y(top)} y2={y(top)} />
      <line class="grid" x1={INSET.left} x2={width - INSET.right} y1={y(0)} y2={y(0)} />
      <path class="area" d={area} />
      <path class="line" d={line} />
      {#if selected !== null}
        <line class="crosshair" x1={x(shownGeneration)} x2={x(shownGeneration)} y1={INSET.top} y2={y(0)} />
        <circle class="marker" cx={x(shownGeneration)} cy={y(shownPopulation)} r="4" />
      {/if}
    </svg>
    <input
      class="reader"
      type="range"
      min="0"
      max={latest}
      value={shownGeneration}
      aria-label="Population at a generation"
      aria-valuetext="{shownPopulation.toLocaleString()} alive at gen {shownGeneration.toLocaleString()}"
      oninput={(event) => (selected = Number(event.currentTarget.value))}
      {onkeydown}
      onblur={() => (selected = null)}
    />
  </div>
  <p class="axis" aria-hidden="true">
    <span>gen 0</span>
    <span>now · gen {latest.toLocaleString()}</span>
  </p>
</figure>

<style>
  .chart {
    display: grid;
    gap: 8px;
  }

  figcaption {
    display: flex;
    gap: 8px;
    align-items: baseline;
    justify-content: space-between;
  }

  .title {
    font-size: 13px;
    font-weight: 600;
  }

  .readout {
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }

  .plot {
    position: relative;
    width: 100%;
    cursor: crosshair;
    border-radius: 6px;
  }

  .plot:has(.reader:focus-visible) {
    outline: 2px solid var(--text);
    outline-offset: 4px;
  }

  .peak {
    position: absolute;
    top: 0;
    right: 6px;
    font-size: 10.5px;
    color: #5f6575;
    pointer-events: none;
  }

  svg {
    display: block;
    overflow: visible;
  }

  .reader {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  .grid {
    stroke: var(--line);
    stroke-width: 1;
  }

  .area {
    fill: rgb(233 235 241 / 10%);
  }

  .line {
    fill: none;
    stroke: #c3c8d4;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .crosshair {
    stroke: rgb(255 255 255 / 30%);
    stroke-width: 1;
  }

  .marker {
    fill: var(--text);
    stroke: #0b0c11;
    stroke-width: 2;
  }

  .axis {
    display: flex;
    justify-content: space-between;
    font-size: 10.5px;
    color: #5f6575;
  }
</style>
