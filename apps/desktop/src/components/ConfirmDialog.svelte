<script lang="ts">
  import type { ConfirmRequest } from '../lib/controller.svelte';

  let { request }: { request: ConfirmRequest } = $props();
  let primary = $state<HTMLButtonElement>();

  $effect(() => {
    primary?.focus();
  });

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      request.resolve('cancel');
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="scrim" role="presentation" onclick={() => request.resolve('cancel')}></div>
<div class="dialog" role="alertdialog" aria-modal="true" aria-labelledby="dlg-title" aria-describedby="dlg-body">
  <h2 id="dlg-title">{request.title}</h2>
  <p id="dlg-body">{request.body}</p>
  <div class="actions">
    {#each request.actions as action (action.id)}
      {#if action.primary}
        <button class="primary" bind:this={primary} onclick={() => request.resolve(action.id)}>{action.label}</button>
      {:else}
        <button class:danger={action.danger} onclick={() => request.resolve(action.id)}>{action.label}</button>
      {/if}
    {/each}
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    background: var(--scrim);
    animation: fade var(--t-med) var(--ease);
    z-index: 20;
  }
  .dialog {
    position: fixed;
    top: 24vh;
    left: 50%;
    transform: translateX(-50%);
    width: min(440px, calc(100% - 48px));
    background: var(--paper-raised);
    border: 1px solid var(--rule-strong);
    border-radius: 6px;
    box-shadow: var(--shadow);
    padding: 24px 24px 18px;
    z-index: 21;
    animation: rise var(--t-med) var(--ease);
  }
  h2 {
    margin: 0 0 6px;
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  p {
    margin: 0;
    color: var(--ink-2);
    font-size: 13px;
    white-space: pre-line;
    overflow-wrap: anywhere;
    user-select: text;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 22px;
    padding-top: 14px;
    border-top: 1px solid var(--rule);
  }
  button {
    padding: 6px 14px;
    border: 1px solid var(--rule-strong);
    border-radius: 3px;
    color: var(--ink-2);
  }
  button:hover {
    color: var(--ink);
    border-color: var(--ink-3);
  }
  .danger {
    margin-right: auto;
    border-color: transparent;
    color: var(--accent-ink);
    padding-left: 0;
  }
  .danger:hover {
    color: var(--accent);
    border-color: transparent;
  }
  .primary {
    background: var(--ink);
    border-color: var(--ink);
    color: var(--paper);
  }
  .primary:hover {
    color: var(--paper);
    border-color: var(--ink);
    opacity: 0.88;
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translate(-50%, 6px);
    }
  }
</style>
