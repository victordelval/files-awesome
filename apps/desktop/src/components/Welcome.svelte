<script lang="ts">
  import { app, formatKeys } from '../lib/controller.svelte';
  import { t } from '../lib/i18n';

  function shortDir(dir: string): string {
    return dir.replace(/^\/home\/[^/]+/, '~');
  }
</script>

<main class="welcome">
  <div class="sheet">
    <header>
      <span class="label">{t.app} · 0.1</span>
      <h1>{t.welcome.tagline}</h1>
    </header>

    <div class="actions">
      <button onclick={() => app.newDoc()}>
        <span>{t.welcome.new}</span><span class="kbd">{formatKeys('Mod+N')}</span>
      </button>
      <button onclick={() => app.open()}>
        <span>{t.welcome.open}</span><span class="kbd">{formatKeys('Mod+O')}</span>
      </button>
    </div>

    <section>
      <div class="label section-head">{t.welcome.recent}</div>
      {#if app.recent.length === 0}
        <p class="muted">{t.welcome.noRecent}</p>
      {:else}
        <ol>
          {#each app.recent as r, i (r.path)}
            <li>
              <button onclick={() => app.openPath(r.path)} title={r.path}>
                <span class="n num">{String(i + 1).padStart(2, '0')}</span>
                <span class="name">{r.name}</span>
                <span class="dir">{shortDir(r.dir)}</span>
              </button>
            </li>
          {/each}
        </ol>
      {/if}
      <p class="muted hint">{t.welcome.drop}</p>
    </section>
  </div>
</main>

<style>
  .welcome {
    grid-row: 1 / -1;
    overflow-y: auto;
    display: grid;
    place-items: start center;
    padding: 18vh 32px 64px;
  }
  .sheet {
    width: min(560px, 100%);
  }
  header {
    padding-bottom: 28px;
    border-bottom: 1px solid var(--ink);
  }
  h1 {
    margin: 14px 0 0;
    font-family: var(--font-serif);
    font-size: 40px;
    font-weight: 400;
    line-height: 1.1;
    letter-spacing: -0.02em;
    color: var(--ink);
  }
  .actions {
    display: grid;
    border-bottom: 1px solid var(--rule);
  }
  .actions button,
  ol button {
    display: flex;
    align-items: baseline;
    gap: 14px;
    width: 100%;
    padding: 12px 2px;
    text-align: left;
    transition: color var(--t-fast) var(--ease), padding var(--t-fast) var(--ease);
  }
  .actions button {
    justify-content: space-between;
    font-size: 15px;
    color: var(--ink);
  }
  .actions button + button {
    border-top: 1px solid var(--rule);
  }
  .actions button:hover,
  ol button:hover {
    padding-left: 8px;
  }
  .actions button:hover span:first-child {
    color: var(--accent-ink);
  }
  section {
    margin-top: 44px;
  }
  .section-head {
    padding-bottom: 8px;
    border-bottom: 1px solid var(--rule);
  }
  ol {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li + li {
    border-top: 1px solid var(--rule);
  }
  ol button {
    padding: 10px 2px;
  }
  .n {
    width: 18px;
    font-size: 10.5px;
    color: var(--ink-3);
  }
  .name {
    color: var(--ink);
    font-size: 13.5px;
    white-space: nowrap;
  }
  .dir {
    flex: 1;
    min-width: 0;
    color: var(--ink-3);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: right;
  }
  .muted {
    color: var(--ink-3);
    font-size: 12.5px;
    margin: 12px 0 0;
  }
  .hint {
    margin-top: 28px;
    color: var(--ink-4);
  }
</style>
