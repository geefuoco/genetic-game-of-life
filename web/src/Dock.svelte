<script lang="ts">
  interface Props {
    running: boolean;
    speed: string;
    docked: boolean;
    ontoggleplay: () => void;
    onstep: () => void;
    onfaster: () => void;
    onslower: () => void;
    onzoomin: () => void;
    onzoomout: () => void;
    ontoggledock: () => void;
  }

  let { running, speed, docked, ontoggleplay, onstep, onfaster, onslower, onzoomin, onzoomout, ontoggledock }: Props = $props();

  let dockToggle: HTMLButtonElement;

  /** Moves focus to the dock toggle, which is recreated whenever the dock moves. */
  export function focusDockToggle(): void {
    dockToggle.focus();
  }

  function onSpeedClick(event: MouseEvent): void {
    if (event.shiftKey) onslower();
    else onfaster();
  }

  function onSpeedMenu(event: MouseEvent): void {
    event.preventDefault();
    onslower();
  }
</script>

<div class="dock" class:glass={!docked} class:docked role="group" aria-label="Simulation controls">
  <div class="group run">
    <button type="button" aria-label={running ? 'Pause' : 'Play'} title={running ? 'Pause (Space)' : 'Play (Space)'} onclick={ontoggleplay}>
      {#if running}
        <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true"
          ><rect x="3.5" y="2.5" width="3" height="11" rx="1" fill="currentColor" /><rect x="9.5" y="2.5" width="3" height="11" rx="1" fill="currentColor" /></svg
        >
      {:else}
        <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true"><path d="M4 2.5v11l9-5.5z" fill="currentColor" /></svg>
      {/if}
    </button>
    <button type="button" aria-label="Step one generation" title="Step one generation (.)" onclick={onstep}>
      <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true"
        ><path d="M3 2.5v11l7-5.5z" fill="currentColor" /><rect x="11" y="2.5" width="2.2" height="11" rx="1" fill="currentColor" /></svg
      >
    </button>
    <button
      type="button"
      class="speed"
      aria-label="Speed {speed}"
      title="Speed {speed}. Click for faster; Shift+click or right-click for slower ([ and ])"
      onclick={onSpeedClick}
      oncontextmenu={onSpeedMenu}>{speed}</button
    >
  </div>
  <span class="sep" aria-hidden="true"></span>
  <div class="group">
    <button type="button" aria-label="Zoom out" title="Zoom out" onclick={onzoomout}>
      <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M3 8h10" /></svg>
    </button>
    <button type="button" aria-label="Zoom in" title="Zoom in" onclick={onzoomin}>
      <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M8 3v10M3 8h10" /></svg>
    </button>
  </div>
  <span class="sep" aria-hidden="true"></span>
  <div class="group">
    <button
      type="button"
      bind:this={dockToggle}
      aria-label={docked ? 'Float the controls' : 'Dock the controls to the side'}
      title={docked ? 'Float the controls (D)' : 'Dock the controls to the side (D)'}
      onclick={ontoggledock}
    >
      {#if docked}
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" aria-hidden="true"
          ><rect x="2" y="3" width="12" height="10" rx="2" /><path d="M5.5 10.5h5" /></svg
        >
      {:else}
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" aria-hidden="true"
          ><rect x="2" y="3" width="12" height="10" rx="2" /><path d="M10 3v10" /></svg
        >
      {/if}
    </button>
  </div>
</div>

<style>
  .dock {
    display: flex;
    gap: 2px;
    align-items: center;
    padding: 6px;
    border-radius: 16px;
  }

  .group {
    display: flex;
    gap: 2px;
    align-items: center;
  }

  button {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    color: #c3c8d4;
    border-radius: 10px;
    transition:
      background 0.12s,
      color 0.12s;
  }

  button:hover {
    color: #fff;
    background: rgb(255 255 255 / 8%);
  }

  .speed {
    width: auto;
    min-width: 44px;
    padding: 0 8px;
    font-size: 13px;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }

  .sep {
    width: 1px;
    height: 22px;
    margin: 0 4px;
    background: rgb(255 255 255 / 8%);
  }

  .docked {
    flex-wrap: wrap;
    row-gap: 4px;
    background: rgb(255 255 255 / 3%);
  }

  .docked .sep {
    display: none;
  }

  .docked .run {
    flex-basis: 100%;
  }
</style>
