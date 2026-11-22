<script lang="ts">
  import { ancestry, byHue, chartX, chartY, CHART_INSET, describeLineage, lineageAt, lineages, type ChartLayout, type Lineage } from './lib/alleleTree.ts';
  import { capitalised } from './lib/colours.ts';
  import { paletteCss } from './lib/palette.ts';
  import type { AlleleId, Census } from './protocol.ts';

  interface Props {
    census: Census;
    /** Chart height in CSS pixels. */
    height: number;
    /** The Allele whose Carrier highlight is shown, if any. */
    highlighted: AlleleId | null;
    /** Called to highlight a lineage's Allele, or to clear the highlight with `null`. */
    onhighlight: (lineage: Lineage | null) => void;
  }

  let { census, height, highlighted, onhighlight }: Props = $props();

  const id = $props.id();
  /** Width of the hue strip and its gap from the chart's left edge, in CSS pixels. */
  const STRIP = { x: 2, width: 4 };
  /** The tip's width, the height kept clear for it above the chart's bottom, and its gap from the point it describes, in CSS pixels. */
  const TIP = { width: 210, height: 90, gap: 14 };

  let canvas: HTMLCanvasElement;
  let width = $state(0);
  let hovered = $state<{ id: AlleleId; x: number; y: number } | null>(null);
  /** The lineage the keyboard reader is on, while it has focus. */
  let reading = $state<AlleleId | null>(null);

  const drawn = $derived(lineages(census));
  const order = $derived(byHue(drawn));
  const layout = $derived<ChartLayout>({ width, height, generation: census.generation });
  const byId = $derived(new Map(drawn.map((lineage) => [lineage.id, lineage])));
  const readIndex = $derived(Math.max(0, order.findIndex((lineage) => lineage.id === reading)));
  const tip = $derived.by(() => {
    if (hovered) {
      const lineage = byId.get(hovered.id);
      return lineage ? { lineage, x: hovered.x, y: hovered.y } : null;
    }
    const lineage = reading === null ? undefined : byId.get(reading);
    return lineage ? { lineage, x: chartX(layout, lineage.lastCarried), y: chartY(layout, lineage.hue) } : null;
  });
  const counts = $derived({
    living: drawn.filter((lineage) => lineage.status === 'living').length,
    hidden: drawn.filter((lineage) => lineage.status === 'hidden').length,
    extinct: drawn.filter((lineage) => lineage.status === 'extinct').length,
  });
  const label = $derived(
    `Colour gene lineages from gen 0 to gen ${census.generation.toLocaleString()}: ` +
      `${counts.living.toLocaleString()} shown, ${counts.hidden.toLocaleString()} hidden everywhere, ${counts.extinct.toLocaleString()} died out`,
  );

  /** Palette colours at every half degree of hue, since a census can hold thousands of Alleles to colour. */
  const STEPS_PER_DEGREE = 2;
  const colours = Array.from({ length: 360 * STEPS_PER_DEGREE }, (_, step) => paletteCss(step / STEPS_PER_DEGREE));
  function colour(hue: number): string {
    return colours[Math.round(hue * STEPS_PER_DEGREE) % colours.length] ?? paletteCss(hue);
  }

  function spoken(lineage: Lineage): string {
    const { name, origin, now } = describeLineage(lineage);
    return `${capitalised(name)}: ${origin}. ${now}.${lineage.id === highlighted ? ' Highlighted.' : ''}`;
  }

  $effect(() => {
    draw(canvas, layout, drawn, highlighted, hovered?.id ?? reading);
  });

  function draw(target: HTMLCanvasElement, at: ChartLayout, all: readonly Lineage[], lit: AlleleId | null, pointed: AlleleId | null): void {
    const ratio = window.devicePixelRatio || 1;
    const [pixelWidth, pixelHeight] = [Math.round(at.width * ratio), Math.round(at.height * ratio)];
    if (target.width !== pixelWidth || target.height !== pixelHeight) {
      target.width = pixelWidth;
      target.height = pixelHeight;
    }
    const context = target.getContext('2d');
    if (!context || at.width === 0) return;
    context.setTransform(ratio, 0, 0, ratio, 0, 0);
    context.clearRect(0, 0, at.width, at.height);
    const plotHeight = at.height - CHART_INSET.top - CHART_INSET.bottom;
    for (let y = 0; y < plotHeight; y += 2) {
      context.fillStyle = colour(Math.round((y / plotHeight) * 360) % 360);
      context.fillRect(STRIP.x, CHART_INSET.top + y, STRIP.width, 2.2);
    }
    const emphasised = lit === null ? null : ancestry(all, lit);
    const isOn = (lineage: Lineage): boolean => (emphasised ? emphasised.has(lineage.id) : lineage.major);
    for (const lineage of all) {
      const on = isOn(lineage) || lineage.id === pointed;
      const strong = lineage.id === pointed || lineage.id === lit;
      const living = lineage.status !== 'extinct';
      const start = chartX(at, lineage.origin);
      const end = Math.max(start + 1, chartX(at, lineage.lastCarried));
      const y = chartY(at, lineage.hue);
      context.strokeStyle = colour(lineage.hue);
      context.globalAlpha = on ? (living ? 0.95 : 0.45) : 0.1;
      if (on && lineage.sourceHue !== null) {
        context.lineWidth = 1;
        context.globalAlpha *= 0.55;
        context.beginPath();
        context.moveTo(start + 0.5, chartY(at, lineage.sourceHue));
        context.lineTo(start + 0.5, y);
        context.stroke();
        context.globalAlpha /= 0.55;
      }
      context.lineWidth = (living ? 1.6 : 1) * (strong ? 2 : 1);
      context.beginPath();
      context.moveTo(start, y);
      context.lineTo(end, y);
      context.stroke();
    }
    const right = chartX(at, at.generation);
    for (const lineage of all) {
      if (lineage.status === 'extinct') continue;
      context.globalAlpha = isOn(lineage) || lineage.id === pointed ? 1 : 0.2;
      context.beginPath();
      context.arc(right, chartY(at, lineage.hue), 1.5 + 6.5 * Math.sqrt(lineage.share), 0, 2 * Math.PI);
      if (lineage.status === 'hidden') {
        context.strokeStyle = colour(lineage.hue);
        context.lineWidth = 1.5;
        context.stroke();
      } else {
        context.fillStyle = colour(lineage.hue);
        context.fill();
      }
    }
    context.globalAlpha = 1;
  }

  function onpointermove(event: PointerEvent): void {
    const bounds = canvas.getBoundingClientRect();
    const point = { x: event.clientX - bounds.left, y: event.clientY - bounds.top };
    const found = lineageAt(drawn, layout, point);
    hovered = found && { id: found.id, ...point };
  }

  function onclick(): void {
    const found = hovered && byId.get(hovered.id);
    if (found) onhighlight(found.id === highlighted ? null : found);
    else if (highlighted !== null) onhighlight(null);
  }

  function onreaderkeydown(event: KeyboardEvent): void {
    if (event.key !== 'Enter' && event.key !== ' ') return;
    event.preventDefault();
    const lineage = order[readIndex];
    if (lineage) onhighlight(lineage.id === highlighted ? null : lineage);
  }
</script>

<figure class="tree" aria-labelledby="{id}-title">
  <figcaption>
    <span id="{id}-title" class="title">Lineage</span>
    <span class="subtitle">each colour gene's hue over time</span>
  </figcaption>
  <div class="plot" bind:clientWidth={width}>
    <p class="visually-hidden" data-testid="lineage-summary">{label}</p>
    <canvas bind:this={canvas} aria-hidden="true" style:height="{height}px" {onpointermove} onpointerleave={() => (hovered = null)} {onclick}></canvas>
    {#if tip}
      {@const { name, origin, now } = describeLineage(tip.lineage)}
      <div
        class="tip"
        aria-hidden="true"
        style:left="{tip.x + TIP.gap + TIP.width > width ? Math.max(0, tip.x - TIP.gap - TIP.width) : tip.x + TIP.gap}px"
        style:top="{Math.max(0, Math.min(height - TIP.height, tip.y + TIP.gap))}px"
      >
        <p class="name"><span class="swatch" style:background={colour(tip.lineage.hue)}></span>{name}</p>
        <p>{origin}</p>
        <p>{now}</p>
        <p class="hint">{tip.lineage.id === highlighted ? 'Click to clear the highlight' : 'Click to light up its carriers'}</p>
      </div>
    {/if}
    {#if order.length > 0}
      <input
        class="reader"
        type="range"
        min="0"
        max={order.length - 1}
        value={readIndex}
        aria-label="Colour gene lineages, by hue"
        aria-valuetext={order[readIndex] ? spoken(order[readIndex]) : ''}
        aria-describedby="{id}-keys"
        oninput={(event) => (reading = order[Number(event.currentTarget.value)]?.id ?? null)}
        onfocus={() => (reading ??= order[readIndex]?.id ?? null)}
        onblur={() => (reading = null)}
        onkeydown={onreaderkeydown}
      />
      <span id="{id}-keys" hidden>Arrow keys move between colour genes. Enter lights up the cells carrying one.</span>
    {/if}
  </div>
  <p class="axis" aria-hidden="true">
    <span>gen 0</span>
    <span>now · gen {census.generation.toLocaleString()}</span>
  </p>
  <p class="legend" aria-hidden="true">
    <span><i class="key line"></i>a colour gene</span>
    <span><i class="key dot"></i>shown now</span>
    <span><i class="key ring"></i>hidden everywhere</span>
  </p>
</figure>

<style>
  .tree {
    display: grid;
    gap: 8px;
  }

  figcaption {
    display: flex;
    gap: 8px;
    align-items: baseline;
  }

  .title {
    font-size: 13px;
    font-weight: 600;
  }

  .subtitle {
    font-size: 12px;
    color: #5f6575;
  }

  .plot {
    position: relative;
    width: 100%;
    border-radius: 6px;
  }

  .plot:has(.reader:focus-visible) {
    outline: 2px solid var(--text);
    outline-offset: 4px;
  }

  canvas {
    display: block;
    width: 100%;
    cursor: pointer;
  }

  .tip {
    position: absolute;
    z-index: 2;
    width: 210px;
    padding: 9px 11px;
    font-size: 12px;
    line-height: 1.5;
    color: #b9becb;
    pointer-events: none;
    background: rgb(10 11 15 / 95%);
    border: 1px solid #23262f;
    border-radius: 10px;
  }

  .tip .name {
    display: flex;
    gap: 7px;
    align-items: center;
    margin-bottom: 2px;
    font-weight: 600;
    color: #fff;
    text-transform: capitalize;
  }

  .tip .hint {
    margin-top: 4px;
    font-size: 11px;
    color: #5f6575;
  }

  .swatch {
    display: inline-block;
    width: 10px;
    height: 10px;
    border-radius: 50%;
  }

  .reader,
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  .axis {
    display: flex;
    justify-content: space-between;
    font-size: 10.5px;
    color: #5f6575;
  }

  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    font-size: 11.5px;
    color: #5f6575;
  }

  .legend span {
    display: inline-flex;
    gap: 5px;
    align-items: center;
  }

  .key {
    display: inline-block;
    width: 9px;
    height: 9px;
    background: var(--muted);
    border-radius: 50%;
  }

  .key.ring {
    background: none;
    border: 1.5px solid var(--muted);
  }

  .key.line {
    width: 14px;
    height: 2px;
    background: linear-gradient(90deg, #7bd, #db7);
    border-radius: 1px;
  }
</style>
