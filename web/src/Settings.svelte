<script lang="ts">
  import { budgetLabel, PEDIGREE_BUDGET_MB } from './lib/pedigree.ts';
  import type { MutationSettings } from './protocol.ts';
  import {
    RULE_PRESETS,
    SIZE_PRESETS,
    densityLabel,
    mutationRateLabel,
    mutationMagnitudeLabel,
    presetOf,
    readSoupDraft,
    type NewSoup,
    type RuleStatus,
    type SoupDraft,
    type SoupField,
  } from './lib/settings.ts';

  interface Props {
    rule: RuleStatus;
    mutation: MutationSettings;
    /** The memory budget of Recorded History, in bytes. */
    pedigreeBudget: number;
    /** The New Soup form, kept by the caller so it survives the settings moving between popover and panel. */
    draft: SoupDraft;
    onrule: (rule: string) => void;
    onmutation: (mutation: MutationSettings) => void;
    onpedigreebudget: (bytes: number) => void;
    onnewsoup: (soup: NewSoup) => void;
    randomSeed: () => bigint;
  }

  let { rule, mutation, pedigreeBudget, draft = $bindable(), onrule, onmutation, onpedigreebudget, onnewsoup, randomSeed }: Props = $props();

  const MB = 1_000_000;

  const id = $props.id();
  const CUSTOM = '';

  let notation = $derived(rule.rejected?.rule ?? rule.running);
  const preset = $derived(presetOf(rule.running));
  let problems = $state<Partial<Record<SoupField, string>>>({});

  function onPreset(event: Event): void {
    const notation = (event.currentTarget as HTMLSelectElement).value;
    if (notation !== CUSTOM) onrule(notation);
  }

  function applyRule(event: SubmitEvent): void {
    event.preventDefault();
    onrule(notation.trim());
  }

  function startSoup(event: SubmitEvent): void {
    event.preventDefault();
    const result = readSoupDraft(draft, randomSeed);
    if (result.ok) {
      problems = {};
      onnewsoup(result.soup);
    } else {
      problems = result.problems;
    }
  }

  function setSize(size: number): void {
    draft.width = size;
    draft.height = size;
  }
</script>

<div class="settings">
  <label class="row choice" for="{id}-rule">
    <span>Rule</span>
    <select id="{id}-rule" value={preset?.notation ?? CUSTOM} onchange={onPreset}>
      {#each RULE_PRESETS as option (option.notation)}
        <option value={option.notation}>{option.label}</option>
      {/each}
      {#if !preset}
        <option value={CUSTOM} disabled>Custom</option>
      {/if}
    </select>
  </label>
  <form class="rule" onsubmit={applyRule}>
    <label class="row choice" for="{id}-rule-text">
      <span>Custom rule</span>
      <input
        id="{id}-rule-text"
        class="text"
        type="text"
        spellcheck="false"
        autocomplete="off"
        bind:value={notation}
        aria-invalid={rule.rejected !== null}
        aria-describedby={rule.rejected ? `${id}-rule-problem` : undefined}
      />
    </label>
    <button type="submit" class="apply">Apply</button>
  </form>
  {#if rule.rejected}
    <p id="{id}-rule-problem" class="problem" role="alert">{rule.rejected.message}</p>
  {/if}

  <label class="row" for="{id}-rate">
    <span>Mutation</span>
    <input
      id="{id}-rate"
      type="range"
      min="0"
      max="60"
      value={Math.round(mutation.rate * 1000)}
      aria-valuetext={mutationRateLabel(mutation.rate)}
      oninput={(event) => onmutation({ ...mutation, rate: Number(event.currentTarget.value) / 1000 })}
    />
    <output for="{id}-rate">{mutationRateLabel(mutation.rate)}</output>
  </label>
  <label class="row" for="{id}-reach">
    <span>Mutation reach</span>
    <input
      id="{id}-reach"
      type="range"
      min="5"
      max="90"
      value={mutation.magnitude}
      aria-valuetext={mutationMagnitudeLabel(mutation.magnitude)}
      oninput={(event) => onmutation({ ...mutation, magnitude: Number(event.currentTarget.value) })}
    />
    <output for="{id}-reach">{mutationMagnitudeLabel(mutation.magnitude)}</output>
  </label>

  <label class="row" for="{id}-budget">
    <span>Ancestry memory</span>
    <input
      id="{id}-budget"
      type="range"
      min={PEDIGREE_BUDGET_MB.min}
      max={PEDIGREE_BUDGET_MB.max}
      step={PEDIGREE_BUDGET_MB.step}
      value={Math.round(pedigreeBudget / MB)}
      aria-valuetext={budgetLabel(pedigreeBudget)}
      aria-describedby="{id}-budget-hint"
      oninput={(event) => onpedigreebudget(Number(event.currentTarget.value) * MB)}
    />
    <output for="{id}-budget">{budgetLabel(pedigreeBudget)}</output>
  </label>
  <p id="{id}-budget-hint" class="hint">Older ancestors are forgotten first once this fills.</p>

  <form class="soup" aria-labelledby="{id}-soup" onsubmit={startSoup} novalidate>
    <h3 id="{id}-soup">New soup</h3>
    <label class="row" for="{id}-alleles">
      <span>Colour genes</span>
      <input id="{id}-alleles" type="range" min="1" max="16" bind:value={draft.foundingAlleles} />
      <output for="{id}-alleles">{draft.foundingAlleles}</output>
    </label>
    <label class="row" for="{id}-density">
      <span>Density</span>
      <input
        id="{id}-density"
        type="range"
        min="1"
        max="100"
        value={Math.round(draft.density * 100)}
        aria-valuetext={densityLabel(draft.density)}
        oninput={(event) => (draft.density = Number(event.currentTarget.value) / 100)}
      />
      <output for="{id}-density">{densityLabel(draft.density)}</output>
    </label>
    <div class="row size">
      <label for="{id}-width">Width</label>
      <input
        id="{id}-width"
        class="text"
        type="number"
        min="16"
        max="1024"
        step="1"
        bind:value={draft.width}
        aria-invalid={problems.width !== undefined}
        aria-describedby={problems.width ? `${id}-width-problem` : undefined}
      />
      <label for="{id}-height">Height</label>
      <input
        id="{id}-height"
        class="text"
        type="number"
        min="16"
        max="1024"
        step="1"
        bind:value={draft.height}
        aria-invalid={problems.height !== undefined}
        aria-describedby={problems.height ? `${id}-height-problem` : undefined}
      />
    </div>
    <div class="presets" role="group" aria-label="Size presets">
      {#each SIZE_PRESETS as size (size)}
        <button
          type="button"
          aria-label="{size} by {size}"
          aria-pressed={draft.width === size && draft.height === size}
          onclick={() => setSize(size)}>{size}</button
        >
      {/each}
    </div>
    {#if problems.width}
      <p id="{id}-width-problem" class="problem">{problems.width}</p>
    {/if}
    {#if problems.height}
      <p id="{id}-height-problem" class="problem">{problems.height}</p>
    {/if}
    <label class="row choice" for="{id}-seed">
      <span>Seed</span>
      <input
        id="{id}-seed"
        class="text"
        type="text"
        inputmode="numeric"
        spellcheck="false"
        autocomplete="off"
        placeholder="random"
        bind:value={draft.seed}
        aria-invalid={problems.seed !== undefined}
        aria-describedby={problems.seed ? `${id}-seed-problem` : undefined}
      />
    </label>
    {#if problems.seed}
      <p id="{id}-seed-problem" class="problem">{problems.seed}</p>
    {/if}
    <button type="submit" class="start">Start new soup</button>
  </form>
</div>

<style>
  .settings {
    display: grid;
    gap: 12px;
    font-size: 12.5px;
    color: var(--muted);
  }

  form {
    display: grid;
    gap: 12px;
  }

  .rule {
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 8px;
  }

  .soup {
    padding-top: 12px;
    border-top: 1px solid var(--line);
  }

  h3 {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }

  .row {
    display: grid;
    grid-template-columns: 1fr 130px 44px;
    gap: 10px;
    align-items: center;
  }

  .row.choice {
    grid-template-columns: 1fr 150px;
  }

  .row.size {
    grid-template-columns: auto 1fr auto 1fr;
  }

  output {
    font-size: 12px;
    color: var(--text);
    text-align: right;
  }

  input[type='range'] {
    width: 100%;
    accent-color: var(--text);
  }

  select,
  .text {
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

  .text[aria-invalid='true'] {
    border-color: #f08a7e;
  }

  select:focus-visible,
  input:focus-visible {
    outline: 2px solid var(--text);
    outline-offset: 1px;
  }

  .hint {
    margin-top: -6px;
    font-size: 11.5px;
  }

  .problem {
    font-size: 12px;
    color: #f08a7e;
  }

  .presets {
    display: flex;
    gap: 6px;
    justify-content: flex-end;
  }

  .presets button,
  .apply,
  .start {
    height: 28px;
    padding: 0 10px;
    color: #c3c8d4;
    background: rgb(255 255 255 / 5%);
    border-radius: 8px;
    transition:
      background 0.12s,
      color 0.12s;
  }

  .presets button:hover,
  .apply:hover,
  .start:hover {
    color: #fff;
    background: rgb(255 255 255 / 10%);
  }

  .presets button[aria-pressed='true'] {
    color: var(--text);
    box-shadow: inset 0 0 0 1px rgb(255 255 255 / 30%);
  }

  .start {
    justify-self: end;
    font-weight: 500;
    color: var(--text);
  }
</style>
