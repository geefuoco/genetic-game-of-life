<script lang="ts">
  import { dominanceLabel, MAX_DOMINANCE } from './lib/brush.ts';
  import type { Rect, Size } from './lib/camera.ts';
  import { capitalised, colourName } from './lib/colours.ts';
  import { paletteCss } from './lib/palette.ts';
  import type { InspectedAllele, Inspection, LineageAllele } from './protocol.ts';

  interface Props {
    inspection: Inspection;
    /** Where the inspected Cell is on the stage, or `null` while it is out of view. */
    anchor: Rect | null;
    /** The stage the card is placed on. */
    stage: Size;
    onclose: () => void;
  }

  let { inspection, anchor, stage, onclose }: Props = $props();

  /** Space between the card and its Cell, and between the card and the stage edges, in CSS pixels. */
  const GAP = 18;
  const EDGE = 12;
  /** Space kept clear at the bottom of the stage for the floating dock. */
  const DOCK_CLEARANCE = 130;

  let width = $state(0);
  let height = $state(0);

  const first = $derived(inspection.alleles[0]);
  const second = $derived(inspection.alleles[1]);
  /** The Alleles in the order the sum reads: a shown one first. */
  const order = $derived(first.shown || !second.shown ? ([first, second] as const) : ([second, first] as const));
  const kind = $derived(first.hue === second.hue ? 'pure' : first.shown && second.shown ? 'blend' : 'carrier');
  const dead = $derived(inspection.deathGeneration !== null);
  /** The Alleles whose lineages are listed: one when both Alleles share their whole lineage. */
  const traced = $derived(sameLineage(first, second) ? [order[0]] : order);
  const subtitle = $derived(
    inspection.deathGeneration !== null
      ? `Died at generation ${inspection.deathGeneration.toLocaleString()}`
      : kind === 'pure'
        ? 'Pure'
        : kind === 'blend'
          ? 'Blend'
          : 'Carries a hidden colour',
  );

  const placement = $derived.by(() => {
    if (!anchor) return null;
    let left = anchor.x + anchor.width + GAP;
    if (left + width > stage.width - EDGE) left = anchor.x - GAP - width;
    if (left < EDGE) left = Math.min(stage.width - width - EDGE, Math.max(EDGE, anchor.x + anchor.width / 2 - width / 2));
    const top = Math.max(EDGE, Math.min(stage.height - height - DOCK_CLEARANCE, anchor.y + anchor.height / 2 - height / 2));
    return { left, top };
  });

  function sameLineage(a: InspectedAllele, b: InspectedAllele): boolean {
    return (
      a.lineage.length === b.lineage.length &&
      a.lineage.every((allele, i) => {
        const other = b.lineage[i];
        return other !== undefined && allele.hue === other.hue && allele.dominance === other.dominance && allele.origin === other.origin;
      })
    );
  }

  function name(allele: { hue: number }): string {
    return colourName(allele.hue);
  }

  /** A Dominance Level as the strength bars describe it, such as "Strength 3 of 4, strong". */
  function dominanceText(allele: InspectedAllele): string {
    return `Strength ${allele.dominance + 1} of ${MAX_DOMINANCE + 1}, ${dominanceLabel(allele.dominance)}`;
  }

  /** When an Allele in a lineage appeared: a Founding Allele at Generation 0, one painted or stamped in later, or a mutant. */
  function origin(allele: LineageAllele, founding: boolean): string {
    if (!founding) return `gen ${allele.origin.toLocaleString()}`;
    return allele.origin === 0 ? 'founding' : `added gen ${allele.origin.toLocaleString()}`;
  }
</script>

{#snippet gene(allele: InspectedAllele)}
  <div class="gene" class:hidden={!allele.shown} style:color={paletteCss(allele.hue)}>
    <span class="swatch" style:background={allele.shown ? paletteCss(allele.hue) : null} aria-hidden="true"></span>
    <span class="name">{name(allele)}</span>
    <span class="state">{allele.shown ? 'shown' : 'hidden'}</span>
    <span class="bars" role="img" aria-label={dominanceText(allele)} title={dominanceText(allele)}>
      {#each { length: MAX_DOMINANCE + 1 }, level (level)}
        <i class:filled={level <= allele.dominance}></i>
      {/each}
    </span>
  </div>
{/snippet}

<section
  class="card glass"
  class:dead
  aria-labelledby="inspector-title"
  style:left="{placement?.left ?? 0}px"
  style:top="{placement?.top ?? 0}px"
  style:visibility={placement ? null : 'hidden'}
  bind:offsetWidth={width}
  bind:offsetHeight={height}
>
  <header>
    <div>
      <h2 id="inspector-title">{capitalised(colourName(inspection.phenotype))} cell</h2>
      <p class="subtitle" aria-live="polite">{subtitle}</p>
    </div>
    <button type="button" class="close" aria-label="Close the inspector" title="Close (Esc)" onclick={onclose}>
      <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true"
        ><path d="M2 2l8 8M10 2l-8 8" /></svg
      >
    </button>
  </header>
  <p class="caption">One colour gene from each parent. Bars show strength.</p>
  <div class="sum" role="group" aria-label="Its colour genes">
    {@render gene(order[0])}
    <span class="operator" aria-hidden="true">+</span>
    {@render gene(order[1])}
    <span class="operator" aria-hidden="true">=</span>
    <span class="result" style:background={paletteCss(inspection.phenotype)} role="img" aria-label="Shows {colourName(inspection.phenotype)}"></span>
  </div>
  <p class="why">
    {#if kind === 'pure'}
      Both colour genes are <b>{name(first)}</b>, so that is the colour it shows.
    {:else if kind === 'blend'}
      <b>{capitalised(name(first))}</b> and <b>{name(second)}</b> are equally strong, so they mix into
      <b>{colourName(inspection.phenotype)}</b>.
    {:else if name(order[0]) === name(order[1])}
      The stronger <b>{name(order[0])}</b> hides a slightly different {name(order[1])}. Its offspring can still inherit it.
    {:else}
      <b>{capitalised(name(order[0]))}</b> is stronger, so it hides <b>{name(order[1])}</b>. Its offspring can still inherit and show
      {name(order[1])}.
    {/if}
  </p>
  <section class="lineages" aria-labelledby="inspector-lineages">
    <h3 id="inspector-lineages">Where its colour genes came from</h3>
    {#each traced as allele, gene (gene)}
      <ol class="lineage" aria-label="Lineage of the {name(allele)} colour gene, oldest first">
        {#each [...allele.lineage].reverse() as link, i (i)}
          {@const extinct = link.status === 'extinct'}
          <li class:extinct>
            <span class="dot" style:background={paletteCss(link.hue)} aria-hidden="true"></span>
            {name(link)}
            <span class="when">{origin(link, i === 0)}{extinct ? ', died out' : ''}</span>
          </li>
        {/each}
      </ol>
    {/each}
  </section>
  <p class="born">Born at generation {inspection.birthGeneration.toLocaleString()}</p>
</section>

<style>
  .card {
    position: absolute;
    z-index: 5;
    width: 276px;
    max-height: calc(100% - 24px);
    padding: 14px;
    overflow: auto;
    border-radius: 16px;
    transition: opacity 0.2s;
  }

  .card.dead {
    opacity: 0.62;
  }

  header {
    display: flex;
    gap: 10px;
    align-items: flex-start;
  }

  h2 {
    font-size: 14px;
    font-weight: 600;
  }

  .subtitle {
    margin-top: 1px;
    font-size: 12px;
    color: var(--muted);
  }

  .close {
    display: grid;
    flex: none;
    place-items: center;
    width: 26px;
    height: 26px;
    margin-left: auto;
    color: var(--muted);
    border-radius: 8px;
  }

  .close:hover {
    color: #fff;
    background: rgb(255 255 255 / 8%);
  }

  .caption {
    margin: 12px 0 8px;
    font-size: 11.5px;
    color: var(--muted);
  }

  .sum {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .gene {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 5px;
    align-items: center;
    padding: 10px 4px 9px;
    background: rgb(255 255 255 / 3.5%);
    border-radius: 12px;
  }

  .swatch {
    width: 22px;
    height: 22px;
    border-radius: 50%;
  }

  .gene.hidden .swatch {
    border: 2px dashed currentcolor;
  }

  .name {
    font-size: 12px;
    font-weight: 500;
    color: var(--text);
    text-transform: capitalize;
  }

  .state {
    font-size: 10px;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.09em;
  }

  .bars {
    display: flex;
    gap: 2px;
  }

  .bars i {
    width: 9px;
    height: 3px;
    background: rgb(255 255 255 / 14%);
    border-radius: 2px;
  }

  .bars i.filled {
    background: #dfe3ec;
  }

  .operator {
    width: 10px;
    font-size: 15px;
    color: rgb(255 255 255 / 35%);
    text-align: center;
  }

  .result {
    flex: none;
    width: 40px;
    height: 40px;
    border-radius: 11px;
  }

  .why {
    margin-top: 12px;
    font-size: 12.5px;
    line-height: 1.5;
    color: #c4c8d3;
  }

  .why b {
    font-weight: 600;
    color: #fff;
  }

  .lineages {
    display: grid;
    gap: 6px;
    margin-top: 12px;
  }

  h3 {
    font-size: 11.5px;
    font-weight: 500;
    color: var(--muted);
  }

  .lineage {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 0;
    padding: 0;
    font-size: 11.5px;
    list-style: none;
  }

  .lineage li {
    display: flex;
    gap: 4px;
    align-items: center;
    text-transform: capitalize;
  }

  .lineage li + li::before {
    margin: 0 5px;
    color: rgb(255 255 255 / 35%);
    content: '→' / '';
  }

  .lineage li.extinct {
    color: var(--muted);
  }

  .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
  }

  .extinct .dot {
    opacity: 0.45;
  }

  .when {
    color: var(--muted);
    text-transform: none;
  }

  .born {
    margin-top: 10px;
    font-size: 11px;
    color: var(--muted);
  }
</style>
