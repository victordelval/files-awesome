<script lang="ts">
  import { app, formatKeys, type Command } from '../lib/controller.svelte';
  import { t } from '../lib/i18n';

  let query = $state('');
  let index = $state(0);
  let input = $state<HTMLInputElement>();

  /** Subsequence match with a bonus for word starts — small and predictable. */
  function score(label: string, q: string): number {
    if (!q) return 1;
    const l = label.toLowerCase().normalize('NFD').replace(/\p{M}/gu, '');
    const s = q.toLowerCase().normalize('NFD').replace(/\p{M}/gu, '');
    let pos = 0;
    let total = 0;
    for (const ch of s) {
      const found = l.indexOf(ch, pos);
      if (found < 0) return 0;
      total += found === 0 || l[found - 1] === ' ' ? 3 : found === pos ? 2 : 1;
      pos = found + 1;
    }
    return total;
  }

  const results = $derived(
    app.commands
      .filter((c) => c.id !== 'palette' && (c.when?.() ?? true))
      .map((c) => ({ c, s: score(c.label, query) }))
      .filter((r) => r.s > 0)
      .sort((a, b) => b.s - a.s)
      .map((r) => r.c),
  );

  $effect(() => {
    input?.focus();
  });

  $effect(() => {
    void query;
    index = 0;
  });

  function close() {
    app.paletteOpen = false;
    app.focusEditor();
  }

  function run(c: Command | undefined) {
    if (!c) return;
    app.paletteOpen = false;
    void c.run();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      index = Math.min(results.length - 1, index + 1);
    } else if (e.key === 'ArrowUp') {
      index = Math.max(0, index - 1);
    } else if (e.key === 'Enter') {
      run(results[index]);
    } else if (e.key === 'Escape') {
      close();
    } else {
      return;
    }
    e.preventDefault();
    e.stopPropagation();
  }
</script>

<div class="scrim" role="presentation" onclick={close}></div>
<div class="palette" role="dialog" aria-label={t.cmd.palette}>
  <input
    bind:this={input}
    bind:value={query}
    onkeydown={onKey}
    placeholder={t.cmd.placeholder}
    spellcheck="false"
    autocomplete="off"
    aria-controls="palette-list"
  />
  <ul id="palette-list" role="listbox">
    {#each results as c, i (c.id)}
      <li role="option" aria-selected={i === index}>
        <button class:sel={i === index} onmouseenter={() => (index = i)} onclick={() => run(c)}>
          <span>{c.label}</span>
          {#if c.keys?.[0]}<span class="kbd">{formatKeys(c.keys[0])}</span>{/if}
        </button>
      </li>
    {:else}
      <li class="none">{t.cmd.empty}</li>
    {/each}
  </ul>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    background: var(--scrim);
    z-index: 10;
    animation: fade var(--t-fast) var(--ease);
  }
  .palette {
    position: fixed;
    top: 16vh;
    left: 50%;
    transform: translateX(-50%);
    width: min(560px, calc(100% - 48px));
    background: var(--paper-raised);
    border: 1px solid var(--rule-strong);
    border-radius: 6px;
    box-shadow: var(--shadow);
    z-index: 11;
    overflow: hidden;
    animation: rise var(--t-med) var(--ease);
  }
  input {
    width: 100%;
    font: inherit;
    font-size: 15px;
    color: var(--ink);
    background: transparent;
    border: 0;
    border-bottom: 1px solid var(--rule);
    padding: 16px 20px;
    outline: none;
  }
  input::placeholder {
    color: var(--ink-4);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 6px;
    max-height: 50vh;
    overflow-y: auto;
  }
  button {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 14px;
    border-radius: 4px;
    color: var(--ink-2);
    text-align: left;
  }
  button.sel {
    background: var(--paper-sunk);
    color: var(--ink);
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .none {
    padding: 14px;
    color: var(--ink-3);
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translate(-50%, -6px);
    }
  }
</style>
