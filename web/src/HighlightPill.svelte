<script lang="ts">
  import { capitalised, colourName } from './lib/colours.ts';
  import { paletteCss } from './lib/palette.ts';

  interface Props {
    /** Hue of the highlighted Allele. */
    hue: number;
    /** Live Cells showing it, and Carriers hiding it, or `null` until a frame reports them. */
    counts: { shown: number; hidden: number } | null;
    onclear: () => void;
  }

  let { hue, counts, onclear }: Props = $props();

  const colour = $derived(paletteCss(hue));
</script>

<section class="pill glass" aria-label="Highlight" style:--allele={colour}>
  <span class="swatch" aria-hidden="true"></span>
  <b>{capitalised(colourName(hue))} colour gene</b>
  {#if counts && counts.shown + counts.hidden === 0}
    <span class="count">carried by none</span>
  {:else if counts}
    <span class="count"><i class="fill" aria-hidden="true"></i>{counts.shown.toLocaleString()} show it</span>
    <span class="count"><i class="ring" aria-hidden="true"></i>{counts.hidden.toLocaleString()} carry it hidden</span>
  {/if}
  <button type="button" aria-label="Clear the highlight" title="Clear the highlight (Esc)" onclick={onclear}>
    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true"
      ><path d="M2 2l8 8M10 2l-8 8" /></svg
    >
  </button>
</section>

<style>
  .pill {
    position: absolute;
    top: 18px;
    left: 50%;
    z-index: 5;
    display: flex;
    gap: 14px;
    align-items: center;
    max-width: calc(100% - 32px);
    padding: 6px 6px 6px 14px;
    font-size: 12.5px;
    color: #c3c8d4;
    white-space: nowrap;
    border-radius: 999px;
    transform: translateX(-50%);
  }

  b {
    margin-left: -8px;
    font-weight: 600;
    color: #fff;
  }

  .swatch {
    width: 10px;
    height: 10px;
    background: var(--allele);
    border-radius: 50%;
  }

  .count {
    display: flex;
    gap: 6px;
    align-items: center;
    font-variant-numeric: tabular-nums;
  }

  .fill,
  .ring {
    display: inline-block;
    width: 11px;
    height: 11px;
    border-radius: 3px;
  }

  .fill {
    background: var(--allele);
  }

  .ring {
    border: 2px solid var(--allele);
  }

  button {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    color: var(--muted);
    border-radius: 50%;
  }

  button:hover {
    color: #fff;
    background: rgb(255 255 255 / 8%);
  }
</style>
