<script lang="ts">
  import { onMount } from 'svelte';
  import { app } from '../lib/controller.svelte';

  let host = $state<HTMLDivElement>();

  onMount(() => {
    if (host) app.mountEditor(host);
    return () => app.unmountEditor();
  });
</script>

<div class="fa-editor" bind:this={host}></div>
