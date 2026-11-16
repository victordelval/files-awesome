<script lang="ts">
  import { app } from '../lib/controller.svelte';
  import { t } from '../lib/i18n';

  const top = $derived(Math.min(...app.outline.map((h) => h.level)));
</script>

<nav class="outline" aria-label={t.outline}>
  <div class="label head">{t.outline}</div>
  {#if app.outline.length === 0}
    <p class="empty">{t.outlineEmpty}</p>
  {:else}
    <ol>
      {#each app.outline as h, i (h.from)}
        <li style:--depth={h.level - top}>
          <button
            class:active={i === app.activeHeading}
            class:top={h.level === top}
            onclick={() => app.goTo(h.from)}
            title={h.text}
          >
            {#if h.num}<span class="n num">{h.num}</span>{/if}
            <span class="txt">{h.text}</span>
          </button>
        </li>
      {/each}
    </ol>
  {/if}
</nav>

<style>
  .outline {
    pointer-events: auto;
    max-height: 100%;
    overflow-y: auto;
    padding: var(--page-top) 20px 48px 32px;
    scrollbar-width: none;
  }
  .head {
    padding-bottom: 10px;
    margin-bottom: 8px;
    border-bottom: 1px solid var(--rule);
  }
  .empty {
    margin: 0;
    color: var(--ink-4);
    font-size: 12px;
  }
  ol {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    padding-left: calc(var(--depth) * 12px + 24px);
  }
  li:has(.top) {
    padding-left: 0;
  }
  button {
    position: relative;
    display: flex;
    gap: 8px;
    width: 100%;
    text-align: left;
    padding: 4px 0;
    color: var(--ink-3);
    font-size: 12px;
    line-height: 1.35;
    transition: color var(--t-fast) var(--ease);
  }
  button.top {
    color: var(--ink-2);
    padding-top: 7px;
  }
  button:hover {
    color: var(--ink);
  }
  button.active {
    color: var(--ink);
  }
  button.active::before {
    content: '';
    position: absolute;
    left: -14px;
    top: 50%;
    width: 8px;
    height: 1px;
    background: var(--accent);
  }
  button.top.active::before {
    top: calc(50% + 1.5px);
  }
  .n {
    flex: 0 0 auto;
    width: 16px;
    color: var(--ink-3);
    font-size: 10.5px;
    padding-top: 1px;
  }
  .txt {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
