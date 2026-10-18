<script lang="ts">
  import { tick } from "svelte";
  import Grid from "./views/Grid.svelte";
  import Player from "./views/Player.svelte";

  let view: "grid" | "player" = $state("grid");
  // Grid stays mounted (hidden) while the Player is open, so its filters survive; scroll and focus are restored by hand.
  let scrollY = 0;
  let opener: HTMLElement | null = null;

  async function openPlayer() {
    scrollY = window.scrollY;
    opener = document.activeElement instanceof HTMLElement && document.activeElement !== document.body ? document.activeElement : null;
    view = "player";
    await tick();
    document.querySelector<HTMLElement>("#player-view h1")?.focus({ preventScroll: true });
  }

  async function closePlayer() {
    view = "grid";
    await tick();
    window.scrollTo(0, scrollY);
    if (opener?.isConnected) opener.focus({ preventScroll: true });
    else document.querySelector<HTMLElement>("#grid-view .tile, #grid-view .chip")?.focus({ preventScroll: true });
  }
</script>

<div id="grid-view" class="view" hidden={view !== "grid"}>
  <Grid onopenplayer={openPlayer} />
</div>
{#if view === "player"}
  <div id="player-view" class="view">
    <Player onback={closePlayer} />
  </div>
{/if}

<style>
  .view { animation: view-in var(--motion-duration-base) var(--motion-easing) both; }
  .view[hidden] { display: none; }
  @keyframes view-in { from { opacity: 0; transform: translateY(8px); } }
  @media (prefers-reduced-motion: reduce) {
    .view { animation: none; }
  }
</style>
