<script lang="ts">
  import Grid from "./views/Grid.svelte";
  import Player from "./views/Player.svelte";

  let view: "grid" | "player" = $state("grid");
</script>

{#if view === "grid"}
  <Grid onopenplayer={() => (view = "player")} />
{:else}
  <Player onback={() => (view = "grid")} />
{/if}
