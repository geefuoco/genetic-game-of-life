<script lang="ts">
  import { MAX_DOMINANCE, PATTERNS, dominanceLabel, type Brush } from './lib/brush.ts';
  import { paletteCss } from './lib/palette.ts';
  import type { Allele, PatternName } from './protocol.ts';

  interface Props {
    /** The brush, kept by the caller so it survives the settings moving between popover and panel. */
    brush: Brush;
  }

  let { brush = $bindable() }: Props = $props();

  const id = $props.id();
  /** The hue slider's track: the palette sampled every 30°, so it shows exactly the colours that will appear. */
  const HUE_TRACK = `linear-gradient(90deg, ${Array.from({ length: 13 }, (_, i) => paletteCss((i * 30) % 360)).join(', ')})`;

  function setAllele(which: 'colour' | 'carried', change: Partial<Allele>): void {
    brush[which] = { ...brush[which], ...change };
  }
</script>

{#snippet allele(which: 'colour' | 'carried', name: string, dominanceName: string)}
  {@const value = brush[which]}
  <label class="row" for="{id}-{which}-hue">
    <span>{name}</span>
    <input
      id="{id}-{which}-hue"
      class="hue"
      type="range"
      min="0"
      max="359"
      value={value.hue}
      style:--track={HUE_TRACK}
      aria-valuetext="hue {value.hue}°"
      oninput={(event) => setAllele(which, { hue: Number(event.currentTarget.value) })}
    />
    <output for="{id}-{which}-hue"><span class="swatch" style:background={paletteCss(value.hue)}></span></output>
  </label>
  <label class="row" for="{id}-{which}-dominance">
    <span>{dominanceName}</span>
    <input
      id="{id}-{which}-dominance"
      type="range"
      min="0"
      max={MAX_DOMINANCE}
      value={value.dominance}
      aria-valuetext={dominanceLabel(value.dominance)}
      oninput={(event) => setAllele(which, { dominance: Number(event.currentTarget.value) })}
    />
    <output for="{id}-{which}-dominance">{dominanceLabel(value.dominance)}</output>
  </label>
{/snippet}

<section class="brush" aria-labelledby="{id}-title">
  <h3 id="{id}-title">Brush</h3>
  {@render allele('colour', 'Colour', 'Dominance')}
  <label class="check" for="{id}-carrier">
    <input id="{id}-carrier" type="checkbox" bind:checked={brush.carrier} />
    <span>Carrier brush <small>paints a second colour gene</small></span>
  </label>
  {#if brush.carrier}
    {@render allele('carried', 'Second colour', 'Second dominance')}
  {/if}
  <label class="row choice" for="{id}-pattern">
    <span>Pattern</span>
    <select id="{id}-pattern" value={brush.pattern} onchange={(event) => (brush.pattern = event.currentTarget.value as PatternName)}>
      {#each PATTERNS as option (option.name)}
        <option value={option.name}>{option.label}</option>
      {/each}
    </select>
  </label>
</section>

<style>
  .brush {
    display: grid;
    gap: 12px;
    padding-top: 12px;
    font-size: 12.5px;
    color: var(--muted);
    border-top: 1px solid var(--line);
  }

  h3 {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }

  .row {
    display: grid;
    grid-template-columns: 1fr 130px 60px;
    gap: 10px;
    align-items: center;
  }

  .row.choice {
    grid-template-columns: 1fr 150px;
  }

  output {
    display: flex;
    justify-content: flex-end;
    font-size: 12px;
    color: var(--text);
  }

  input[type='range'] {
    width: 100%;
    accent-color: var(--text);
  }

  .hue {
    height: 8px;
    background: var(--track);
    border-radius: 4px;
    appearance: none;
  }

  .hue::-webkit-slider-thumb {
    width: 14px;
    height: 14px;
    background: #fff;
    border-radius: 50%;
    box-shadow: 0 0 0 2px rgb(0 0 0 / 50%);
    appearance: none;
  }

  .hue::-moz-range-thumb {
    width: 14px;
    height: 14px;
    background: #fff;
    border: 0;
    border-radius: 50%;
  }

  .swatch {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    box-shadow: 0 0 0 2px rgb(255 255 255 / 22%);
  }

  .check {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .check input {
    accent-color: var(--text);
  }

  small {
    margin-left: 4px;
    font-size: 12px;
  }

  select {
    width: 100%;
    min-width: 0;
    height: 28px;
    padding: 0 8px;
    font: inherit;
    color: var(--text);
    background: rgb(255 255 255 / 5%);
    border: 1px solid var(--line);
    border-radius: 8px;
  }

  select:focus-visible,
  input:focus-visible {
    outline: 2px solid var(--text);
    outline-offset: 1px;
  }
</style>
