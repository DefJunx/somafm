<script lang="ts">
  let { onopenplayer }: { onopenplayer?: () => void } = $props();
</script>

<main>
  <h1>Grid</h1>
  <button onclick={() => onopenplayer?.()}>Open player</button>
</main>

<style>
  main {
    padding: var(--size-4);
  }
</style>
