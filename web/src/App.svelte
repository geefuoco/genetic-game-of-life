<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { MediaQuery } from 'svelte/reactivity';
  import AlleleTree from './AlleleTree.svelte';
  import Brush from './Brush.svelte';
  import Dock, { type DockLayout } from './Dock.svelte';
  import HighlightPill from './HighlightPill.svelte';
  import Inspector from './Inspector.svelte';
  import Notice from './Notice.svelte';
  import Settings from './Settings.svelte';
  import Share from './Share.svelte';
  import Stats from './Stats.svelte';
  import WorldView from './WorldView.svelte';
  import { DEFAULT_BRUSH, toolGenotype, type Brush as BrushSettings, type Tool } from './lib/brush.ts';
  import { AUTOSAVE_MS, browserWorldStore } from './lib/autosave.ts';
  import { browserStorage, loadDocked, saveDocked } from './lib/dock.ts';
  import { keyAction, type KeyAction } from './lib/keys.ts';
  import type { NewSoup, RuleStatus, SoupDraft } from './lib/settings.ts';
  import { currentDevice, defaultPedigreeBudget, MAX_PEDIGREE_DEPTH, PEDIGREE_DEPTH, PEDIGREE_DEPTH_STEP } from './lib/pedigree.ts';
  import { DEFAULT_SPEED, SPEEDS, cycleSpeed, nudgeSpeed } from './lib/speed.ts';
  import { paletteCss } from './lib/palette.ts';
  import { linkInFragment, openingPlan, rejectionNotice, shareUrl, snapshotFileName, type Opening } from './lib/persistence.ts';
  import type { Rect } from './lib/camera.ts';
  import type { AlleleId, Census, CellPosition, Frame, Inspection, MutationSettings, Pedigree, SnapshotPurpose, SoupSettings, WorldSource } from './protocol.ts';
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
  /** Viewports at most this wide, in CSS pixels, keep the controls in a drawer. */
  const NARROW_PX = 640;
  /** What a visitor is told once a Snapshot they chose is running. */
  const IMPORTED = "Snapshot imported. Ancestors restart from here: a snapshot doesn't keep the ones before it.";

  let docked = $state(loadDocked(browserStorage()));
  const narrowViewport = new MediaQuery(`max-width: ${NARROW_PX}px`);
  const touchScreen = new MediaQuery('pointer: coarse');
  /** Whether the controls drawer is open, on a viewport narrow enough to have one. */
  let drawerOpen = $state(false);
  const narrow = $derived(narrowViewport.current);
  /** Whether the side panel is showing: docked beside the World, or as the drawer over it on narrow viewports. */
  const panelOpen = $derived(narrow ? drawerOpen : docked);
  /** Whether the controls float over the World, with the Lineage card and settings popover they open. */
  const floating = $derived(!narrow && !docked);
  let running = $state(true);
  let speed = $state(DEFAULT_SPEED);
  let generation = $state(0);
  let population = $state(0);
  /** The latest census, while the side panel or the Lineage card that shows it is open. */
  let census = $state.raw<Census | null>(null);
  let lineageOpen = $state(false);
  /** The Allele whose Carrier highlight is shown, with its hue. */
  let highlighted = $state.raw<{ id: AlleleId; hue: number } | null>(null);
  /** How many live Cells show and hide the highlighted Allele, as of the latest frame that reports them. */
  let highlightCounts = $state.raw<{ shown: number; hidden: number } | null>(null);
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
  /** The inspected Organism's Pedigree as last sent, with the depth it was asked for. */
  let pedigree = $state.raw<{ pedigree: Pedigree; depth: number } | null>(null);
  let pedigreeBudget = $state(defaultPedigreeBudget(currentDevice()));
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
  /** The controls, of which only one set is shown at a time. */
  let dock: Dock | undefined = $state();
  let popover: HTMLElement | undefined = $state();
  let simulation: Simulation | null = null;
  const worldStore = browserWorldStore();
  /** The ways to open the first World still to try, while none has opened. */
  let opening: Opening[] = [];
  /** Whether a World has opened, so a refused Share Link or Snapshot leaves one running. */
  let opened = false;
  /** Whether the World or its settings may have changed since the last autosave, so an unchanged World is not autosaved again. */
  let changedSinceAutosave = false;
  /** The core's Share Link of the run the World is on, or `null` when none can reproduce it. */
  let shareLink = $state<string | null>(null);
  let notice = $state.raw<{ kind: 'info' | 'error'; text: string } | null>(null);

  const hint = $derived(touchScreen.current ? 'tap a cell to see its colour genes · drag to move · pinch to zoom' : 'click a cell to see its colour genes · drag to move · scroll to zoom');
  const stats = $derived(`gen ${generation.toLocaleString()} · ${population.toLocaleString()} alive`);
  const brushColours = $derived([paletteCss(brush.first.hue), paletteCss(brush.second.hue)] as const);
  const inspected = $derived(inspection && { x: inspection.x, y: inspection.y, dead: inspection.deathGeneration !== null });
  /** The Pedigree, once it is the inspected Organism's: one asked for before another Organism was inspected may still arrive. */
  const inspectedPedigree = $derived.by(() => {
    const root = pedigree?.pedigree.organisms[0];
    if (!pedigree || !inspection || !root) return null;
    const same = root.x === inspection.x && root.y === inspection.y && root.birthGeneration === inspection.birthGeneration;
    return same ? pedigree : null;
  });
  const expandable = $derived(
    inspectedPedigree !== null &&
      inspectedPedigree.depth < MAX_PEDIGREE_DEPTH &&
      inspectedPedigree.pedigree.organisms.some((organism) => organism.origin === 'unexplored'),
  );

  function onFrame(frame: Frame): void {
    changedSinceAutosave = true;
    generation = frame.generation;
    population = frame.population;
    canUndo = frame.canUndo;
    canRedo = frame.canRedo;
    inspection = following ? frame.inspection : null;
    if (highlighted && frame.highlight?.allele === highlighted.id) highlightCounts = { shown: frame.highlight.shown, hidden: frame.highlight.hidden };
    view.present(frame);
  }

  /** Shows the Carrier highlight of an Allele, or clears it with `null`. */
  function highlight(allele: { id: AlleleId; hue: number } | null): void {
    highlighted = allele && { id: allele.id, hue: allele.hue };
    highlightCounts = null;
    if (allele) hintShown = false;
    simulation?.send({ type: 'highlight', allele: allele?.id ?? null });
  }

  function inspect(cell: CellPosition | null): void {
    if (!cell) {
      if (inspection) closeInspector();
      return;
    }
    const [x, y] = cell;
    following = true;
    hintShown = false;
    simulation?.send({ type: 'inspect', x, y });
    simulation?.send({ type: 'pedigree', depth: PEDIGREE_DEPTH });
  }

  function expandPedigree(): void {
    if (!inspectedPedigree) return;
    simulation?.send({ type: 'pedigree', depth: Math.min(MAX_PEDIGREE_DEPTH, inspectedPedigree.depth + PEDIGREE_DEPTH_STEP) });
  }

  function setPedigreeBudget(bytes: number): void {
    pedigreeBudget = bytes;
    simulation?.send({ type: 'set-pedigree-budget', bytes });
  }

  function closeInspector(): void {
    following = false;
    inspection = null;
    pedigree = null;
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
    changedSinceAutosave = true;
    simulation?.send({ type: 'set-mutation', mutation: next });
  }

  /** Tries the next way to open the first World. */
  function openNext(): void {
    const next = opening.shift();
    if (!next || !simulation) return;
    const rate = SPEEDS[speed]!.rate;
    if (next.from === 'link') simulation.send({ type: 'open-link', link: next.link, rate, running });
    else if (next.from === 'autosave') simulation.send({ type: 'import', ...next.autosaved, source: 'autosave', rate, running });
    else simulation.send({ type: 'start', soup: { ...OPENING_SOUP, seed: randomSeed() }, rate, running });
  }

  function clearAddressLink(): void {
    if (location.hash !== '') history.replaceState(history.state, '', location.pathname + location.search);
  }

  /** Opens the Share Link in the address, if there is one, and takes it out of the address so a reload resumes the autosaved World. */
  function openAddressLink(): void {
    const link = linkInFragment(location.hash);
    if (link === '') return;
    clearAddressLink();
    simulation?.send({ type: 'open-link', link, rate: SPEEDS[speed]!.rate, running });
  }

  function onWorld(world: { source: WorldSource; rule: string; mutation: MutationSettings }): void {
    opening = [];
    opened = true;
    rule = { running: world.rule, rejected: null };
    mutation = world.mutation;
    highlighted = null;
    highlightCounts = null;
    if (world.source === 'snapshot') notice = { kind: 'info', text: IMPORTED };
    else if (notice?.kind === 'info') notice = null;
    changedSinceAutosave = true;
    autosave();
  }

  function onRejected(rejected: { source: Exclude<WorldSource, 'soup'>; message: string }): void {
    if (rejected.source !== 'autosave') notice = { kind: 'error', text: rejectionNotice(rejected.source, rejected.message, opened) };
    if (!opened) openNext();
  }

  function onSnapshot(taken: { purpose: SnapshotPurpose; snapshot: Uint8Array; generation: number; link: string | null }): void {
    if (taken.purpose === 'autosave') {
      void worldStore.save({ snapshot: taken.snapshot, link: taken.link });
      return;
    }
    const url = URL.createObjectURL(new Blob([taken.snapshot as Uint8Array<ArrayBuffer>], { type: 'application/octet-stream' }));
    const download = document.createElement('a');
    download.href = url;
    download.download = snapshotFileName(taken.generation);
    download.click();
    setTimeout(() => URL.revokeObjectURL(url), 0);
  }

  function exportSnapshot(): void {
    simulation?.send({ type: 'take-snapshot', purpose: 'export' });
  }

  async function importSnapshot(file: File): Promise<void> {
    const snapshot = new Uint8Array(await file.arrayBuffer());
    simulation?.send({ type: 'import', snapshot, link: null, source: 'snapshot', rate: SPEEDS[speed]!.rate, running });
  }

  function autosave(): void {
    if (!opened || !changedSinceAutosave) return;
    changedSinceAutosave = false;
    simulation?.send({ type: 'take-snapshot', purpose: 'autosave' });
  }

  function newSoup(soup: NewSoup): void {
    highlighted = null;
    highlightCounts = null;
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
    dock?.focusSettingsToggle();
  }

  /**
   * Docks or floats the controls, or on a narrow viewport opens or closes
   * the drawer. Focus follows a drawer, so it is never left behind in one
   * that has closed.
   */
  async function toggleDock(refocus: boolean): Promise<void> {
    if (narrow) {
      drawerOpen = !drawerOpen;
    } else {
      docked = !docked;
      saveDocked(browserStorage(), docked);
    }
    if (refocus || narrow) {
      await tick();
      dock?.focusDockToggle();
    }
  }

  function toggleLineage(): void {
    lineageOpen = !lineageOpen;
  }

  /** Whether the side panel or the Lineage card shows the census. */
  const censusShown = $derived(panelOpen || (floating && lineageOpen));

  /** Asks for census events only while the side panel or the Lineage card shows them. */
  function watchCensus(watching: boolean): void {
    if (!watching) census = null;
    simulation?.send({ type: 'watch-census', watching });
  }

  $effect(() => {
    watchCensus(censusShown);
  });

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
    if (event.key === 'Escape' && narrow && drawerOpen) {
      event.preventDefault();
      void toggleDock(true);
      return;
    }
    if (event.key === 'Escape' && settingsOpen && floating) {
      event.preventDefault();
      void closeSettings();
      return;
    }
    if (event.key === 'Escape' && inspection) {
      event.preventDefault();
      closeInspector();
      return;
    }
    if (event.key === 'Escape' && highlighted) {
      event.preventDefault();
      highlight(null);
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
    simulation = new Simulation({
      frame: onFrame,
      census: (next) => {
        if (censusShown) census = next;
      },
      rule: (running) => {
        rule = { running, rejected: null };
        changedSinceAutosave = true;
      },
      pedigree: (next, depth) => {
        if (following) pedigree = next && { pedigree: next, depth };
      },
      ruleRejected: (rejected) => {
        rule = { running: rule.running, rejected };
      },
      world: onWorld,
      rejected: onRejected,
      shareLink: (link) => {
        shareLink = link;
      },
      snapshot: onSnapshot,
      error: (message) => {
        problem = message;
      },
    });
    simulation.send({ type: 'set-pedigree-budget', bytes: pedigreeBudget });
    const fragment = location.hash;
    void worldStore.load().then((autosaved) => {
      opening = openingPlan(fragment, autosaved);
      clearAddressLink();
      openNext();
    });
    watchCensus(censusShown);
    const hintTimer = setTimeout(() => {
      hintShown = false;
    }, HINT_MS);
    const autosaving = setInterval(autosave, AUTOSAVE_MS);
    const onHashChange = (): void => {
      if (opened) openAddressLink();
    };
    const onVisibilityChange = (): void => {
      if (document.visibilityState === 'hidden') autosave();
    };
    window.addEventListener('hashchange', onHashChange);
    document.addEventListener('visibilitychange', onVisibilityChange);
    return () => {
      clearTimeout(hintTimer);
      clearInterval(autosaving);
      window.removeEventListener('hashchange', onHashChange);
      document.removeEventListener('visibilitychange', onVisibilityChange);
      simulation?.dispose();
      simulation = null;
    };
  });
</script>

<svelte:window {onkeydown} {onkeyup} />

{#snippet controls(layout: DockLayout)}
  <Dock
    bind:this={dock}
    {running}
    speed={SPEEDS[speed]!.label}
    {layout}
    {settingsOpen}
    {lineageOpen}
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
    ontogglelineage={toggleLineage}
  />
{/snippet}

{#snippet lineage(height: number)}
  {#if census}
    <AlleleTree {census} {height} highlighted={highlighted?.id ?? null} onhighlight={highlight} />
  {/if}
{/snippet}

{#snippet settings()}
  <Settings
    {rule}
    {mutation}
    {pedigreeBudget}
    bind:draft
    onrule={setRule}
    onmutation={setMutation}
    onpedigreebudget={setPedigreeBudget}
    onnewsoup={newSoup}
    {randomSeed}
  />
  <Brush bind:brush />
  <Share url={shareLink && shareUrl(location.href, shareLink)} onexport={exportSnapshot} onimport={importSnapshot} />
{/snippet}

{#if problem}
  <p class="problem" role="alert">{problem}</p>
{/if}

<div class="app" class:docked={docked && !narrow} class:narrow hidden={problem !== null}>
  <main class="stage" bind:clientWidth={stageWidth} bind:clientHeight={stageHeight}>
    <WorldView
      bind:this={view}
      bind:anchor
      {tool}
      {inspected}
      oninspect={inspect}
      onsupport={(ok) => (supported = ok)}
      {highlighted}
      onrelease={(buffers) => simulation?.send({ type: 'release', ...buffers })}
      onstroke={stroke}
      onstamp={stamp}
      onstrokeend={() => simulation?.send({ type: 'end-stroke' })}
    />
    <div class="vignette"></div>
    {#if inspection}
      <Inspector
        {inspection}
        {anchor}
        stage={{ width: stageWidth, height: stageHeight }}
        pedigree={inspectedPedigree?.pedigree ?? null}
        {expandable}
        onexpand={expandPedigree}
        onclose={closeInspector}
        highlighted={highlighted?.id ?? null}
        onhighlight={highlight}
      />
    {/if}
    {#if notice}
      <Notice kind={notice.kind} text={notice.text} ondismiss={() => (notice = null)} />
    {/if}
    {#if highlighted}
      <HighlightPill hue={highlighted.hue} counts={highlightCounts} onclear={() => highlight(null)} />
    {/if}
    <header class="wordmark" aria-hidden={docked && !narrow}>
      <h1>genetic life</h1>
      <p data-testid="stats">{stats}</p>
    </header>
    <p class="hint" class:gone={!hintShown}>{hint}</p>
    {#if narrow && !drawerOpen}
      <div class="floating">{@render controls('bar')}</div>
    {/if}
    {#if floating}
      {#if lineageOpen}
        <div class="lineage-card glass" id="lineage-card">{@render lineage(210)}</div>
      {/if}
      <div class="floating">{@render controls('floating')}</div>
      {#if settingsOpen}
        <div class="popover glass" id="settings-popover" role="dialog" aria-label="Settings" bind:this={popover}>{@render settings()}</div>
      {/if}
    {/if}
  </main>
  <aside class="panel" class:drawer={narrow} id="side-panel" aria-label={narrow ? 'Controls drawer' : 'Side panel'} hidden={!panelOpen}>
    <div class="panel-inner">
      <header class="panel-head">
        <h2>genetic life</h2>
        <p>{stats}</p>
      </header>
      {#if panelOpen}
        {@render controls(narrow ? 'drawer' : 'docked')}
        <Stats {generation} {population} {census} />
        {@render lineage(300)}
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

  .lineage-card {
    position: absolute;
    top: 18px;
    right: 18px;
    z-index: 4;
    width: 344px;
    max-width: calc(100% - 36px);
    min-height: 60px;
    padding: 14px 14px 10px;
    border-radius: 16px;
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

  .panel.drawer {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    z-index: 10;
    width: min(var(--panel-width), calc(100% - 40px));
    box-shadow: -12px 0 40px rgb(0 0 0 / 45%);
    animation: slide-in 0.28s cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  .drawer .panel-inner {
    width: 100%;
  }

  @keyframes slide-in {
    from {
      transform: translateX(100%);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .panel.drawer {
      animation: none;
    }
  }

  .narrow .hint {
    top: 68px;
    right: 26px;
    left: 26px;
    white-space: normal;
    transform: none;
  }

  .narrow .floating {
    bottom: max(16px, env(safe-area-inset-bottom));
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
