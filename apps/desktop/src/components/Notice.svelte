<script lang="ts">
  import { app } from '../lib/controller.svelte';
  import { t } from '../lib/i18n';
</script>

<div class="notice" role="alert">
  <span class="mark" aria-hidden="true"></span>
  {#if app.notice === 'modified'}
    <p>{t.notice.modified}</p>
    <button onclick={() => app.reloadFromDisk()}>{t.notice.reload}</button>
    <button class="primary" onclick={() => app.keepMine()}>{t.notice.keepMine}</button>
  {:else if app.notice === 'conflict'}
    <p>{t.notice.conflict}</p>
    <button onclick={() => app.reloadFromDisk()}>{t.notice.reload}</button>
    <button class="primary" onclick={() => app.overwrite()}>{t.notice.overwrite}</button>
  {:else if app.notice === 'missing'}
    <p>{t.notice.missing}</p>
    <button onclick={() => (app.notice = null)}>{t.notice.dismiss}</button>
    <button class="primary" onclick={() => app.save()}>{t.notice.save}</button>
  {/if}
</div>

<style>
  .notice {
    position: absolute;
    top: 16px;
    left: 50%;
    transform: translateX(-50%);
    width: min(calc(100% - 64px), 760px);
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px 10px 16px;
    background: var(--paper-raised);
    border: 1px solid var(--rule-strong);
    border-radius: 4px;
    box-shadow: var(--shadow);
    font-size: 12.5px;
    animation: drop var(--t-med) var(--ease);
    z-index: 5;
  }
  .mark {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
    flex: 0 0 auto;
  }
  p {
    margin: 0;
    flex: 1;
    color: var(--ink);
  }
  button {
    padding: 5px 10px;
    border: 1px solid var(--rule-strong);
    border-radius: 3px;
    color: var(--ink-2);
    white-space: nowrap;
  }
  button:hover {
    color: var(--ink);
    border-color: var(--ink-3);
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
  @keyframes drop {
    from {
      opacity: 0;
      transform: translate(-50%, -6px);
    }
  }
</style>
