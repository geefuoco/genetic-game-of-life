<script lang="ts" module>
  /**
   * Where the controls are: floating over the World, docked to the side
   * panel, in the drawer that holds the side panel on narrow screens, or in
   * the small bar that opens that drawer and holds only the run controls.
   */
  export type DockLayout = 'floating' | 'docked' | 'drawer' | 'bar';
</script>

<script lang="ts">
  import type { Tool } from './lib/brush.ts';

  interface Props {
    running: boolean;
    speed: string;
    layout: DockLayout;
    settingsOpen: boolean;
    /** Whether the floating Lineage card is open. */
    lineageOpen: boolean;
    tool: Tool;
    /** CSS colours of the brush's first and second Alleles. */
    brushColours: readonly [string, string];
    canUndo: boolean;
    canRedo: boolean;
    ontool: (tool: Tool) => void;
    onundo: () => void;
    onredo: () => void;
    ontoggleplay: () => void;
    onstep: () => void;
    onfaster: () => void;
    onslower: () => void;
    onzoomin: () => void;
    onzoomout: () => void;
    /** Called to move the controls: dock or float them, or open or close the drawer. */
    ontoggledock: () => void;
    ontogglesettings: () => void;
    ontogglelineage: () => void;
  }

  let {
    running,
    speed,
    layout,
    settingsOpen,
    lineageOpen,
    tool,
    brushColours,
    canUndo,
    canRedo,
    ontool,
    onundo,
    onredo,
    ontoggleplay,
    onstep,
    onfaster,
    onslower,
    onzoomin,
    onzoomout,
    ontoggledock,
    ontogglesettings,
    ontogglelineage,
  }: Props = $props();

  /** The name and tooltip of the button that moves the controls, in each layout. */
  const PLACEMENT: Record<DockLayout, { label: string; title: string }> = {
    floating: { label: 'Dock the controls to the side', title: 'Dock the controls to the side (D)' },
    docked: { label: 'Float the controls', title: 'Float the controls (D)' },
    drawer: { label: 'Close the controls', title: 'Close the controls (Esc)' },
    bar: { label: 'Open the controls', title: 'Open the controls (D)' },
  };

  const inPanel = $derived(layout === 'docked' || layout === 'drawer');
  const carrierDot = $derived(`linear-gradient(90deg, ${brushColours[0]} 50%, ${brushColours[1]} 50%)`);

  let dockToggle: HTMLButtonElement;
  let settingsToggle: HTMLButtonElement | undefined = $state();

  /** Moves focus to the settings button, which only the floating dock has. */
  export function focusSettingsToggle(): void {
    settingsToggle?.focus();
  }

  /** Moves focus to the button that moves the controls, which is recreated whenever they move. */
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

<div class="dock" class:glass={!inPanel} class:docked={inPanel} role="group" aria-label="Simulation controls">
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
  {#if layout !== 'bar'}
    <span class="sep" aria-hidden="true"></span>
    <div class="group" role="group" aria-label="Tools">
      <button type="button" aria-label="Look" title="Look: drag to move, click a cell to inspect" aria-pressed={tool === 'look'} class:on={tool === 'look'} onclick={() => ontool('look')}>
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"
          ><path d="M8 1.5v13M1.5 8h13M6 3.5l2-2 2 2M6 12.5l2 2 2-2M3.5 6l-2 2 2 2M12.5 6l2 2-2 2" /></svg
        >
      </button>
      <button type="button" aria-label="Paint" title="Paint: drag to paint cells" aria-pressed={tool === 'paint'} class:on={tool === 'paint'} onclick={() => ontool('paint')}>
        <span class="dot" style:background={brushColours[0]} aria-hidden="true"></span>
      </button>
      <button
        type="button"
        aria-label="Carrier brush"
        title="Carrier brush: drag to paint cells carrying both brush colours"
        aria-pressed={tool === 'carrier'}
        class:on={tool === 'carrier'}
        onclick={() => ontool('carrier')}
      >
        <span class="dot" style:background={carrierDot} aria-hidden="true"></span>
      </button>
      <button type="button" aria-label="Erase" title="Erase: drag to erase cells" aria-pressed={tool === 'erase'} class:on={tool === 'erase'} onclick={() => ontool('erase')}>
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round" aria-hidden="true"
          ><path d="M9.5 2.5l4 4-6.5 6.5H4.5l-2-2z" /><path d="M6 6l4 4M7 13h6.5" stroke-linecap="round" /></svg
        >
      </button>
      <button type="button" aria-label="Stamp a pattern" title="Stamp: click to stamp the chosen pattern" aria-pressed={tool === 'stamp'} class:on={tool === 'stamp'} onclick={() => ontool('stamp')}>
        <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true"
          ><rect x="6.5" y="2" width="3" height="3" rx="0.8" /><rect x="11" y="6.5" width="3" height="3" rx="0.8" /><rect x="2" y="11" width="3" height="3" rx="0.8" /><rect
            x="6.5"
            y="11"
            width="3"
            height="3"
            rx="0.8"
          /><rect x="11" y="11" width="3" height="3" rx="0.8" /></svg
        >
      </button>
      <button type="button" aria-label="Undo" title="Undo (Ctrl+Z)" disabled={!canUndo} onclick={onundo}>
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"
          ><path d="M5.5 3L2.5 6l3 3" /><path d="M2.5 6h7a4 4 0 010 8H7" /></svg
        >
      </button>
      <button type="button" aria-label="Redo" title="Redo (Ctrl+Shift+Z)" disabled={!canRedo} onclick={onredo}>
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"
          ><path d="M10.5 3l3 3-3 3" /><path d="M13.5 6h-7a4 4 0 000 8H9" /></svg
        >
      </button>
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
  {/if}
  <span class="sep" aria-hidden="true"></span>
  <div class="group">
    {#if layout === 'floating'}
      <button
        type="button"
        class:on={lineageOpen}
        aria-label="Lineage"
        title="Lineage: each colour gene's hue over time"
        aria-expanded={lineageOpen}
        aria-controls="lineage-card"
        onclick={ontogglelineage}
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"
          ><path d="M2 8h3.5M5.5 8 8 4h6M5.5 8 8 12h6" /></svg
        >
      </button>
      <button
        type="button"
        bind:this={settingsToggle}
        class:on={settingsOpen}
        aria-label="Settings"
        title="Settings"
        aria-expanded={settingsOpen}
        aria-controls="settings-popover"
        onclick={ontogglesettings}
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" aria-hidden="true"
          ><circle cx="8" cy="8" r="2.2" /><path
            d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4"
          /></svg
        >
      </button>
    {/if}
    <button
      type="button"
      bind:this={dockToggle}
      aria-label={PLACEMENT[layout].label}
      title={PLACEMENT[layout].title}
      aria-controls={layout === 'bar' ? 'side-panel' : undefined}
      onclick={ontoggledock}
    >
      {#if layout === 'bar'}
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true"
          ><path d="M2.5 4h11M2.5 8h11M2.5 12h11" /></svg
        >
      {:else if layout === 'drawer'}
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true"
          ><path d="M4 4l8 8M12 4l-8 8" /></svg
        >
      {:else if layout === 'docked'}
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
    flex-wrap: wrap;
    gap: 2px;
    align-items: center;
    justify-content: center;
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

  button:hover:enabled,
  button.on {
    color: #fff;
    background: rgb(255 255 255 / 8%);
  }

  button:disabled {
    cursor: default;
    opacity: 0.35;
  }

  .dot {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    box-shadow: 0 0 0 2px rgb(255 255 255 / 22%);
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
