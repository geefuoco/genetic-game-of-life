<script lang="ts">
  import { onMount, tick } from 'svelte';
  import Dock from './Dock.svelte';
  import WorldView from './WorldView.svelte';
  import { browserStorage, loadDocked, saveDocked } from './lib/dock.ts';
  import { keyAction, type KeyAction } from './lib/keys.ts';
  import { DEFAULT_SPEED, SPEEDS, cycleSpeed, nudgeSpeed } from './lib/speed.ts';
  import type { Frame, SoupSettings } from './protocol.ts';
  import { Simulation } from './simulation.ts';

  /** The Soup the page opens on, apart from its random Seed. */
  const OPENING_SOUP: Omit<SoupSettings, 'seed'> = { width: 160, height: 160, density: 0.3, foundingAlleles: 8 };
  /** How long the hint stays before fading, in ms. */
  const HINT_MS = 9000;
  /** Zoom factor of one press of a zoom button. */
  const ZOOM_STEP = 1.25;

  let docked = $state(loadDocked(browserStorage()));
  let running = $state(true);
  let speed = $state(DEFAULT_SPEED);
  let generation = $state(0);
  let population = $state(0);
  let hintShown = $state(true);
  let problem = $state<string | null>(null);
  let supported = false;
  let view: WorldView;
  let dock: Dock;
  let simulation: Simulation | null = null;

  const stats = $derived(`gen ${generation.toLocaleString()} · ${population.toLocaleString()} alive`);

  function onFrame(frame: Frame): void {
    generation = frame.generation;
    population = frame.population;
    view.present(frame);
  }

  function togglePlay(): void {
    running = !running;
    simulation?.send({ type: running ? 'play' : 'pause' });
  }

  function step(): void {
    running = false;
    simulation?.send({ type: 'step' });
  }

  function setSpeed(index: number): void {
    speed = index;
    simulation?.send({ type: 'set-rate', rate: SPEEDS[index]!.rate });
  }

  async function toggleDock(refocus: boolean): Promise<void> {
    docked = !docked;
    saveDocked(browserStorage(), docked);
    if (refocus) {
      await tick();
      dock.focusDockToggle();
    }
  }

  function isEditable(target: EventTarget | null): boolean {
    return target instanceof Element && target.closest('input, textarea, select, [contenteditable]') !== null;
  }

  function actionOf(event: KeyboardEvent): KeyAction | null {
    return keyAction({ key: event.key, ctrlKey: event.ctrlKey, metaKey: event.metaKey, altKey: event.altKey, editable: isEditable(event.target) });
  }

  function onkeydown(event: KeyboardEvent): void {
    const action = actionOf(event);
    if (!action) return;
    event.preventDefault();
    if (event.repeat && (action === 'toggle-play' || action === 'toggle-dock')) return;
    if (action === 'toggle-play') togglePlay();
    else if (action === 'step') step();
    else if (action === 'slower') setSpeed(nudgeSpeed(speed, -1));
    else if (action === 'faster') setSpeed(nudgeSpeed(speed, 1));
    else void toggleDock(false);
  }

  function onkeyup(event: KeyboardEvent): void {
    if (actionOf(event) === 'toggle-play') event.preventDefault();
  }

  onMount(() => {
    if (!supported) {
      problem = 'Genetic Life needs WebGL2, which this browser does not support or has turned off.';
      return;
    }
    const seed = crypto.getRandomValues(new BigUint64Array(1))[0] ?? 0n;
    simulation = new Simulation({
      frame: onFrame,
      error: (message) => {
        problem = message;
      },
    });
    simulation.send({ type: 'start', soup: { ...OPENING_SOUP, seed }, rate: SPEEDS[speed]!.rate, running });
    const hint = setTimeout(() => {
      hintShown = false;
    }, HINT_MS);
    return () => {
      clearTimeout(hint);
      simulation?.dispose();
      simulation = null;
    };
  });
</script>

<svelte:window {onkeydown} {onkeyup} />

{#snippet controls()}
  <Dock
    bind:this={dock}
    {running}
    speed={SPEEDS[speed]!.label}
    {docked}
    ontoggleplay={togglePlay}
    onstep={step}
    onfaster={() => setSpeed(cycleSpeed(speed, 1))}
    onslower={() => setSpeed(cycleSpeed(speed, -1))}
    onzoomin={() => view.zoom(ZOOM_STEP)}
    onzoomout={() => view.zoom(1 / ZOOM_STEP)}
    ontoggledock={() => toggleDock(true)}
  />
{/snippet}

{#if problem}
  <p class="problem" role="alert">{problem}</p>
{/if}

<div class="app" class:docked hidden={problem !== null}>
  <main class="stage">
    <WorldView bind:this={view} onsupport={(ok) => (supported = ok)} onrelease={(phenotypes) => simulation?.send({ type: 'release', phenotypes })} />
    <div class="vignette"></div>
    <header class="wordmark" aria-hidden={docked}>
      <h1>genetic life</h1>
      <p data-testid="stats">{stats}</p>
    </header>
    <p class="hint" class:gone={!hintShown}>drag to move · scroll to zoom</p>
    {#if !docked}
      <div class="floating">{@render controls()}</div>
    {/if}
  </main>
  <aside class="panel" aria-label="Side panel" hidden={!docked}>
    <div class="panel-inner">
      <header class="panel-head">
        <h2>genetic life</h2>
        <p>{stats}</p>
      </header>
      {#if docked}
        {@render controls()}
      {/if}
    </div>
  </aside>
</div>

<style>
  .app {
    position: fixed;
    inset: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 0;
    transition: grid-template-columns 0.34s cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  .app.docked {
    grid-template-columns: minmax(0, 1fr) var(--panel-width);
  }

  .app[hidden] {
    display: none;
  }

  .stage {
    position: relative;
    overflow: hidden;
  }

  .vignette {
    position: absolute;
    inset: 0;
    pointer-events: none;
    background: radial-gradient(120% 100% at 50% 50%, transparent 58%, rgb(0 0 0 / 55%));
  }

  .wordmark {
    position: absolute;
    top: 22px;
    left: 26px;
    pointer-events: none;
    transition: opacity 0.2s;
  }

  .docked .wordmark {
    opacity: 0;
  }

  h1,
  h2 {
    font-size: 15px;
    font-weight: 500;
    letter-spacing: 0.01em;
  }

  .wordmark p,
  .panel-head p {
    margin-top: 2px;
    font-size: 12px;
    color: var(--muted);
  }

  .hint {
    position: absolute;
    top: 22px;
    left: 50%;
    font-size: 12px;
    color: var(--muted);
    white-space: nowrap;
    pointer-events: none;
    transform: translateX(-50%);
    transition: opacity 0.6s;
  }

  .hint.gone {
    opacity: 0;
  }

  .floating {
    position: absolute;
    bottom: 32px;
    left: 50%;
    transform: translateX(-50%);
  }

  .panel {
    display: block;
    overflow: hidden;
    background: #0b0c11;
    border-left: 1px solid var(--line);
  }

  .panel-inner {
    display: flex;
    flex-direction: column;
    gap: 22px;
    width: var(--panel-width);
    height: 100%;
    padding: 22px 20px;
    overflow: auto;
  }

  .problem {
    position: fixed;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 24px;
    font-size: 15px;
    text-align: center;
  }
</style>
