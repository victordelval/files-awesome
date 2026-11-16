<script lang="ts">
  import { app } from '../lib/controller.svelte';
</script>

<div class="region" aria-live="polite">
  {#if app.toast}
    {#key app.toast.id}
      <div class="toast" class:error={app.toast.tone === 'error'}>{app.toast.text}</div>
    {/key}
  {/if}
</div>

<style>
  .region {
    position: fixed;
    left: 50%;
    bottom: calc(var(--titleblock-h) + 20px);
    transform: translateX(-50%);
    z-index: 30;
    pointer-events: none;
  }
  .toast {
    max-width: min(560px, calc(100vw - 48px));
    padding: 8px 14px;
    background: var(--ink);
    color: var(--paper);
    border-radius: 4px;
    font-size: 12px;
    box-shadow: var(--shadow);
    animation: in var(--t-med) var(--ease);
    user-select: text;
  }
  .toast.error {
    border-left: 3px solid var(--accent);
  }
  @keyframes in {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
</style>
