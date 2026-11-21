<script lang="ts">
  import { onMount, tick } from 'svelte';
  import Brush from './Brush.svelte';
  import Dock from './Dock.svelte';
  import Inspector from './Inspector.svelte';
  import Settings from './Settings.svelte';
  import WorldView from './WorldView.svelte';
  import { DEFAULT_BRUSH, toolGenotype, type Brush as BrushSettings, type Tool } from './lib/brush.ts';
  import { browserStorage, loadDocked, saveDocked } from './lib/dock.ts';
  import { keyAction, type KeyAction } from './lib/keys.ts';
  import type { NewSoup, RuleStatus, SoupDraft } from './lib/settings.ts';
  import { DEFAULT_SPEED, SPEEDS, cycleSpeed, nudgeSpeed } from './lib/speed.ts';
  import { paletteCss } from './lib/palette.ts';
  import type { Rect } from './lib/camera.ts';
  import type { CellPosition, Frame, Inspection, MutationSettings, SoupSettings } from './protocol.ts';
  import { Simulation } from './simulation.ts';

  /** The Soup the page opens on, apart from its random Seed: Conway's Life with the core's default Mutation. */
  const OPENING_SOUP: Omit<SoupSettings, 'seed'> = {
    rule: 'B3/S23',
    width: 160,
    height: 160,
    density: 0.3,
    foundingAlleles: 8,
    mutation: { rate: 0.01, magnitude: 30 },
  };
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
  let settingsOpen = $state(false);
  let tool = $state<Tool>('look');
  let brush = $state<BrushSettings>(structuredClone(DEFAULT_BRUSH));
  let canUndo = $state(false);
  let canRedo = $state(false);
  let inspection = $state.raw<Inspection | null>(null);
  /** Whether frames' inspections are wanted: false once the inspector is closed, so frames already on their way cannot reopen it. */
  let following = false;
  let anchor = $state.raw<Rect | null>(null);
  let stageWidth = $state(0);
  let stageHeight = $state(0);
  let rule = $state.raw<RuleStatus>({ running: OPENING_SOUP.rule, rejected: null });
  let mutation = $state.raw<MutationSettings>(OPENING_SOUP.mutation);
  let draft = $state<SoupDraft>({
    foundingAlleles: OPENING_SOUP.foundingAlleles,
    density: OPENING_SOUP.density,
    width: OPENING_SOUP.width,
    height: OPENING_SOUP.height,
    seed: '',
  });
  let supported = false;
  let view: WorldView;
  let dock: Dock;
  let popover: HTMLElement | undefined = $state();
  let simulation: Simulation | null = null;

  const stats = $derived(`gen ${generation.toLocaleString()} · ${population.toLocaleString()} alive`);
  const brushColours = $derived([paletteCss(brush.first.hue), paletteCss(brush.second.hue)] as const);
  const inspected = $derived(inspection && { x: inspection.x, y: inspection.y, dead: inspection.deathGeneration !== null });

  function onFrame(frame: Frame): void {
    generation = frame.generation;
    population = frame.population;
    canUndo = frame.canUndo;
    canRedo = frame.canRedo;
    inspection = following ? frame.inspection : null;
    view.present(frame);
  }

  function inspect([x, y]: CellPosition): void {
    following = true;
    hintShown = false;
    simulation?.send({ type: 'inspect', x, y });
  }

  function closeInspector(): void {
    following = false;
    inspection = null;
    simulation?.send({ type: 'stop-inspecting' });
  }

  function stroke(cells: CellPosition[]): void {
    if (tool === 'erase') simulation?.send({ type: 'erase', cells });
    else simulation?.send({ type: 'paint', cells, genotype: $state.snapshot(toolGenotype(tool, brush)) });
  }

  function stamp([x, y]: CellPosition): void {
    simulation?.send({ type: 'stamp', pattern: brush.pattern, x, y, genotype: $state.snapshot(toolGenotype('stamp', brush)) });
  }

  function undo(): void {
    simulation?.send({ type: 'undo' });
  }

  function redo(): void {
    simulation?.send({ type: 'redo' });
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

  function randomSeed(): bigint {
    return crypto.getRandomValues(new BigUint64Array(1))[0] ?? 0n;
  }

  function setRule(notation: string): void {
    simulation?.send({ type: 'set-rule', rule: notation });
  }

  function setMutation(next: MutationSettings): void {
    mutation = next;
    simulation?.send({ type: 'set-mutation', mutation: next });
  }

  function newSoup(soup: NewSoup): void {
    simulation?.send({ type: 'start', soup: { ...soup, rule: rule.running, mutation }, rate: SPEEDS[speed]!.rate, running });
  }

  async function toggleSettings(): Promise<void> {
    if (settingsOpen) return closeSettings();
    settingsOpen = true;
    await tick();
    popover?.querySelector<HTMLElement>('select, input, button')?.focus();
  }

  async function closeSettings(): Promise<void> {
    settingsOpen = false;
    await tick();
    dock.focusSettingsToggle();
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

  /** Whether focus is in a text field, which handles its own undo and redo. */
  function isTyping(target: EventTarget | null): boolean {
    if (target instanceof HTMLInputElement) return !['range', 'checkbox', 'radio', 'button', 'submit', 'reset', 'color', 'file'].includes(target.type);
    return target instanceof Element && target.closest('textarea, [contenteditable]') !== null;
  }

  function actionOf(event: KeyboardEvent): KeyAction | null {
    return keyAction({
      key: event.key,
      ctrlKey: event.ctrlKey,
      metaKey: event.metaKey,
      altKey: event.altKey,
      shiftKey: event.shiftKey,
      editable: isEditable(event.target),
      typing: isTyping(event.target),
    });
  }

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape' && settingsOpen && !docked) {
      event.preventDefault();
      void closeSettings();
      return;
    }
    if (event.key === 'Escape' && inspection) {
      event.preventDefault();
      closeInspector();
      return;
    }
    const action = actionOf(event);
    if (!action) return;
    event.preventDefault();
    if (event.repeat && (action === 'toggle-play' || action === 'toggle-dock')) return;
    if (action === 'toggle-play') togglePlay();
    else if (action === 'step') step();
    else if (action === 'slower') setSpeed(nudgeSpeed(speed, -1));
    else if (action === 'faster') setSpeed(nudgeSpeed(speed, 1));
    else if (action === 'undo') undo();
    else if (action === 'redo') redo();
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
    const seed = randomSeed();
    simulation = new Simulation({
      frame: onFrame,
      rule: (running) => {
        rule = { running, rejected: null };
      },
      ruleRejected: (rejected) => {
        rule = { running: rule.running, rejected };
      },
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
    {settingsOpen}
    {tool}
    {brushColours}
    {canUndo}
    {canRedo}
    ontool={(next) => (tool = next)}
    onundo={undo}
    onredo={redo}
    ontoggleplay={togglePlay}
    onstep={step}
    onfaster={() => setSpeed(cycleSpeed(speed, 1))}
    onslower={() => setSpeed(cycleSpeed(speed, -1))}
    onzoomin={() => view.zoom(ZOOM_STEP)}
    onzoomout={() => view.zoom(1 / ZOOM_STEP)}
    ontoggledock={() => toggleDock(true)}
    ontogglesettings={toggleSettings}
  />
{/snippet}

{#snippet settings()}
  <Settings {rule} {mutation} bind:draft onrule={setRule} onmutation={setMutation} onnewsoup={newSoup} {randomSeed} />
  <Brush bind:brush />
{/snippet}

{#if problem}
  <p class="problem" role="alert">{problem}</p>
{/if}

<div class="app" class:docked hidden={problem !== null}>
  <main class="stage" bind:clientWidth={stageWidth} bind:clientHeight={stageHeight}>
    <WorldView
      bind:this={view}
      bind:anchor
      {tool}
      {inspected}
      oninspect={inspect}
      onsupport={(ok) => (supported = ok)}
      onrelease={(phenotypes) => simulation?.send({ type: 'release', phenotypes })}
      onstroke={stroke}
      onstamp={stamp}
      onstrokeend={() => simulation?.send({ type: 'end-stroke' })}
    />
    <div class="vignette"></div>
    {#if inspection}
      <Inspector {inspection} {anchor} stage={{ width: stageWidth, height: stageHeight }} onclose={closeInspector} />
    {/if}
    <header class="wordmark" aria-hidden={docked}>
      <h1>genetic life</h1>
      <p data-testid="stats">{stats}</p>
    </header>
    <p class="hint" class:gone={!hintShown}>click a cell to see its colour genes · drag to move · scroll to zoom</p>
    {#if !docked}
      <div class="floating">{@render controls()}</div>
      {#if settingsOpen}
        <div class="popover glass" id="settings-popover" role="dialog" aria-label="Settings" bind:this={popover}>{@render settings()}</div>
      {/if}
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
        <section class="panel-settings" aria-labelledby="panel-settings-title">
          <h2 id="panel-settings-title">Settings</h2>
          {@render settings()}
        </section>
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
    width: max-content;
    max-width: calc(100% - 32px);
    transform: translateX(-50%);
  }

  .popover {
    position: absolute;
    bottom: 100px;
    left: 50%;
    width: 340px;
    max-height: calc(100% - 140px);
    padding: 14px 16px;
    overflow: auto;
    border-radius: 14px;
    transform: translateX(-50%);
  }

  .panel-settings h2 {
    margin-bottom: 12px;
    font-size: 13px;
    font-weight: 600;
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
