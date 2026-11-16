<script lang="ts">
  import { onMount } from 'svelte';
  import { backend } from './lib/backend';
  import { app } from './lib/controller.svelte';
  import CommandPalette from './components/CommandPalette.svelte';
  import ConfirmDialog from './components/ConfirmDialog.svelte';
  import Editor from './components/Editor.svelte';
  import Notice from './components/Notice.svelte';
  import Outline from './components/Outline.svelte';
  import TitleBlock from './components/TitleBlock.svelte';
  import Toast from './components/Toast.svelte';
  import Welcome from './components/Welcome.svelte';

  onMount(() => {
    void app.init();
    const onKey = (e: KeyboardEvent) => app.handleKey(e);
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  $effect(() => {
    void backend.setTitle(app.windowTitle);
  });
</script>

<div class="shell" class:focus={app.focusMode} class:ready={app.ready}>
  {#if app.doc}
    <section class="workspace">
      <Editor />
      {#if app.prefs.outline && !app.focusMode}
        <aside class="rail"><Outline /></aside>
      {/if}
      {#if app.notice}
        <Notice />
      {/if}
    </section>
    {#if !app.focusMode}
      <TitleBlock />
    {/if}
  {:else if app.ready}
    <Welcome />
  {/if}

  {#if app.paletteOpen}
    <CommandPalette />
  {/if}
  {#if app.confirm}
    <ConfirmDialog request={app.confirm} />
  {/if}
  <Toast />
</div>

<style>
  .shell {
    height: 100%;
    display: grid;
    grid-template-rows: 1fr auto;
    opacity: 0;
    transition: opacity var(--t-med) var(--ease);
  }
  .shell.ready {
    opacity: 1;
  }
  .workspace {
    position: relative;
    min-height: 0;
  }
  .rail {
    position: absolute;
    top: 0;
    left: 0;
    bottom: 0;
    width: var(--rail);
    pointer-events: none;
  }
  @media (max-width: 1080px) {
    .rail {
      display: none;
    }
  }
</style>
