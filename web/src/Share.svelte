<script lang="ts">
  interface Props {
    /** The address that opens the run the World is on, or `null` when no Share Link can reproduce it. */
    url: string | null;
    onexport: () => void;
    onimport: (file: File) => void;
  }

  let { url, onexport, onimport }: Props = $props();

  const id = $props.id();
  let copied = $state(false);
  let field: HTMLInputElement | undefined = $state();

  async function copy(): Promise<void> {
    if (!url) return;
    field?.select();
    try {
      await navigator.clipboard.writeText(url);
      copied = true;
      setTimeout(() => (copied = false), 1600);
    } catch {
      copied = false;
    }
  }

  function chosen(event: Event & { currentTarget: HTMLInputElement }): void {
    const file = event.currentTarget.files?.[0];
    event.currentTarget.value = '';
    if (file) onimport(file);
  }
</script>

<section class="share" aria-labelledby="{id}-title">
  <h3 id="{id}-title">Share</h3>
  {#if url}
    <div class="row link">
      <label for="{id}-link">Share link</label>
      <input id="{id}-link" class="text" type="text" readonly value={url} bind:this={field} onfocus={(event) => event.currentTarget.select()} />
      <button type="button" class="action" onclick={copy}>{copied ? 'Copied' : 'Copy link'}</button>
    </div>
  {:else}
    <p class="note" data-testid="no-share-link">
      A link can only replay an unedited soup from its start. Export a snapshot to keep or share this world.
    </p>
  {/if}
  <div class="files">
    <button type="button" class="action" onclick={onexport}>Export snapshot</button>
    <label class="action import">
      Import snapshot
      <input type="file" accept=".ggls,application/octet-stream" onchange={chosen} />
    </label>
  </div>
</section>

<style>
  .share {
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

  .row.link {
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 8px;
    align-items: center;
  }

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

  .note {
    font-size: 12px;
  }

  .files {
    display: flex;
    gap: 6px;
    justify-content: flex-end;
  }

  .action {
    display: inline-grid;
    place-items: center;
    height: 28px;
    padding: 0 10px;
    color: #c3c8d4;
    cursor: pointer;
    background: rgb(255 255 255 / 5%);
    border-radius: 8px;
    transition:
      background 0.12s,
      color 0.12s;
  }

  .action:hover {
    color: #fff;
    background: rgb(255 255 255 / 10%);
  }

  .import {
    position: relative;
  }

  .import input {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    opacity: 0;
  }

  .import:focus-within {
    outline: 2px solid var(--text);
    outline-offset: 2px;
  }

  input:focus-visible {
    outline: 2px solid var(--text);
    outline-offset: 1px;
  }
</style>
