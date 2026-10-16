<script lang="ts">
  let { onback }: { onback?: () => void } = $props();
</script>

<main>
  <button onclick={() => onback?.()}>Back</button>
  <h1>Player</h1>
</main>

<style>
  main {
    padding: var(--size-4);
  }
</style>
