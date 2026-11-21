<script lang="ts">
  import PopulationChart from './PopulationChart.svelte';
  import { alleleRows } from './lib/census.ts';
  import { capitalised, colourName } from './lib/colours.ts';
  import { paletteCss } from './lib/palette.ts';
  import type { Census } from './protocol.ts';

  interface Props {
    /** The Generation and population of the latest frame, which census events can trail by a moment. */
    generation: number;
    population: number;
    census: Census | null;
  }

  let { generation, population, census }: Props = $props();

  const id = $props.id();
  /** Most colour genes listed; the rest are counted. */
  const LISTED = 12;

  const genes = $derived(census && alleleRows(census.alleles, LISTED));
  const widest = $derived(Math.max(1, ...(genes?.rows.map((row) => row.carried) ?? [])));

  function count(value: number | undefined): string {
    return value === undefined ? '–' : value.toLocaleString();
  }
</script>

<section class="stats" aria-label="Stats">
  <dl class="figures">
    <div><dt>generation</dt><dd>{count(generation)}</dd></div>
    <div><dt>alive</dt><dd>{count(population)}</dd></div>
    <div><dt>colour genes</dt><dd>{count(genes?.living)}</dd></div>
    <div><dt>hidden everywhere</dt><dd>{count(genes?.fullyHidden)}</dd></div>
  </dl>

  {#if genes && genes.rows.length > 0}
    <div class="genes">
      <h3 id="{id}-genes">Colour genes</h3>
      <p class="key" aria-hidden="true">
        <span><i class="swatch solid"></i>shown</span>
        <span><i class="swatch faint"></i>carried, hidden</span>
        <span><i class="swatch ring"></i>hidden everywhere</span>
      </p>
      <ul aria-labelledby="{id}-genes">
        {#each genes.rows as row, index (index)}
          {@const name = capitalised(colourName(row.hue))}
          {@const colour = paletteCss(row.hue)}
          <li
            style:--gene={colour}
            class:hidden={row.fullyHidden}
            aria-label="{name}: carried by {row.carried.toLocaleString()}, {row.fullyHidden ? 'hidden everywhere' : `shown by ${row.shown.toLocaleString()}`}"
          >
            <span class="chip" aria-hidden="true"></span>
            <span class="name" aria-hidden="true">{name}</span>
            <span class="bar" aria-hidden="true">
              <span class="carried" style:width="{(100 * row.carried) / widest}%"
                ><span class="shown" style:width="{row.carried === 0 ? 0 : (100 * row.shown) / row.carried}%"></span></span
              >
            </span>
            <span class="numbers" aria-hidden="true">{row.fullyHidden ? 'hidden' : row.shown.toLocaleString()} / {row.carried.toLocaleString()}</span>
          </li>
        {/each}
      </ul>
      {#if genes.more > 0}
        <p class="more">and {genes.more.toLocaleString()} more</p>
      {/if}
    </div>
  {/if}

  {#if census}
    <PopulationChart history={census.populationHistory} />
  {/if}
</section>

<style>
  .stats {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .figures {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px 12px;
  }

  .figures div {
    display: flex;
    flex-direction: column-reverse;
  }

  dt {
    font-size: 12px;
    color: var(--muted);
  }

  dd {
    font-size: 22px;
    font-weight: 500;
    font-feature-settings: normal;
    letter-spacing: -0.01em;
  }

  h3 {
    margin-bottom: 6px;
    font-size: 13px;
    font-weight: 600;
  }

  .key {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    margin-bottom: 10px;
    font-size: 11.5px;
    color: var(--muted);
  }

  .key span {
    display: inline-flex;
    gap: 5px;
    align-items: center;
  }

  .swatch {
    flex: none;
    width: 9px;
    height: 9px;
    border-radius: 50%;
  }

  .swatch.solid {
    background: var(--muted);
  }

  .swatch.faint {
    background: rgb(139 145 161 / 35%);
  }

  .swatch.ring {
    border: 1.5px solid var(--muted);
  }

  ul {
    display: grid;
    gap: 7px;
    padding: 0;
    list-style: none;
  }

  li {
    display: grid;
    grid-template-columns: 12px 58px minmax(0, 1fr) auto;
    gap: 8px;
    align-items: center;
    font-size: 12px;
  }

  .chip {
    width: 12px;
    height: 12px;
    background: var(--gene);
    border-radius: 50%;
  }

  .hidden .chip {
    background: transparent;
    border: 2px solid var(--gene);
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .bar {
    display: block;
    height: 8px;
  }

  .carried {
    display: block;
    min-width: 2px;
    height: 100%;
    overflow: hidden;
    background: color-mix(in srgb, var(--gene) 30%, transparent);
    border-radius: 4px;
  }

  .shown {
    display: block;
    height: 100%;
    background: var(--gene);
  }

  .numbers,
  .more {
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }

  .more {
    margin-top: 8px;
  }
</style>
