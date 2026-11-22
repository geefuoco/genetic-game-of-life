<script lang="ts">
  interface Props {
    /** An `error` is announced at once; `info` waits for the screen reader to finish. */
    kind: 'info' | 'error';
    text: string;
    ondismiss: () => void;
  }

  let { kind, text, ondismiss }: Props = $props();
</script>

<div class="notice glass" class:error={kind === 'error'} role={kind === 'error' ? 'alert' : 'status'}>
  <p>{text}</p>
  <button type="button" aria-label="Dismiss" title="Dismiss" onclick={ondismiss}>
    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true"
      ><path d="M2 2l8 8M10 2l-8 8" /></svg
    >
  </button>
</div>

<style>
  .notice {
    position: absolute;
    top: 64px;
    left: 50%;
    z-index: 6;
    display: flex;
    gap: 12px;
    align-items: center;
    width: max-content;
    max-width: min(520px, calc(100% - 32px));
    padding: 8px 8px 8px 16px;
    font-size: 12.5px;
    color: #c3c8d4;
    border-radius: 14px;
    transform: translateX(-50%);
  }

  .notice.error {
    border-color: rgb(240 138 126 / 45%);
  }

  .error p {
    color: #f6b4ab;
  }

  button {
    display: grid;
    flex: none;
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
