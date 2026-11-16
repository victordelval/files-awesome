<!--
  The status bar, drawn as an architectural title block ("cajetín"):
  labelled cells divided by hairlines.
-->
<script lang="ts">
  import { app, formatKeys } from '../lib/controller.svelte';
  import { t } from '../lib/i18n';

  const nf = new Intl.NumberFormat('es-ES');
  const minutes = $derived(Math.max(1, Math.round(app.words / 230)));
  const format = $derived.by(() => {
    const f = app.doc?.format;
    if (!f) return '';
    const eol = f.lineEnding === 'crlf' ? 'CRLF' : 'LF';
    return `UTF-8${f.bom ? ' BOM' : ''} · ${f.mixedLineEndings ? t.status.mixed : ''}${eol}`;
  });
  const saveState = $derived(
    !app.doc?.path && !app.dirty ? t.status.neverSaved : app.dirty ? t.status.unsaved : t.status.saved,
  );
</script>

<footer class="titleblock">
  <div class="cell file">
    <span class="label">{t.status.file}</span>
    <span class="v">
      <span class="name">{app.displayName}</span>
      {#if app.dirty}<i class="dot" aria-label={t.status.unsaved}></i>{/if}
      {#if app.doc?.dir}<span class="dir"><bdi>{app.doc.dir}</bdi></span>{/if}
    </span>
  </div>
  <div class="cell">
    <span class="label">{t.status.words}</span>
    <span class="v num">{nf.format(app.words)}</span>
  </div>
  <div class="cell wide-only">
    <span class="label">{t.status.reading}</span>
    <span class="v num">{minutes} {t.status.min}</span>
  </div>
  <div class="cell wide-only">
    <span class="label">{t.status.format}</span>
    <span class="v" class:warn={app.doc?.format.mixedLineEndings}>{format}</span>
  </div>
  <div class="cell">
    <span class="label">{t.status.state}</span>
    <span class="v" class:accent={app.dirty}>{saveState}</span>
  </div>
  <button class="cell action" onclick={() => app.toggleSource()} title={formatKeys('Mod+/')}>
    <span class="label">{t.status.mode}</span>
    <span class="v">{app.sourceMode ? t.status.source : t.status.preview}</span>
  </button>
  <button class="cell action" onclick={() => (app.paletteOpen = true)}>
    <span class="label">{t.status.commands}</span>
    <span class="v kbd">{formatKeys('Mod+P')}</span>
  </button>
</footer>

<style>
  .titleblock {
    display: flex;
    height: var(--titleblock-h);
    border-top: 1px solid var(--rule);
    background: var(--paper);
  }
  .cell {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 1px;
    padding: 0 16px;
    border-left: 1px solid var(--rule);
    min-width: 0;
    text-align: left;
  }
  .cell:first-child {
    border-left: 0;
    padding-left: 32px;
  }
  .file {
    flex: 1;
  }
  .label {
    font-size: 8.5px;
    letter-spacing: 0.14em;
  }
  .v {
    display: flex;
    align-items: baseline;
    gap: 8px;
    font-size: 11.5px;
    color: var(--ink-2);
    white-space: nowrap;
    overflow: hidden;
  }
  .name {
    color: var(--ink);
    font-weight: 500;
  }
  .dir {
    color: var(--ink-3);
    overflow: hidden;
    text-overflow: ellipsis;
    direction: rtl;
    text-align: left;
  }
  .dot {
    flex: 0 0 auto;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
    align-self: center;
  }
  .accent {
    color: var(--accent-ink);
  }
  .warn {
    color: var(--accent-ink);
  }
  .action {
    transition: background var(--t-fast) var(--ease);
  }
  .action:hover {
    background: var(--paper-sunk);
  }
  .action:hover .v {
    color: var(--ink);
  }
  @media (max-width: 860px) {
    .wide-only {
      display: none;
    }
  }
</style>
