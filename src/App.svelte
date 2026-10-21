<script lang="ts">
  import { tick } from "svelte";
  import Grid from "./views/Grid.svelte";
  import Player from "./views/Player.svelte";
  import Liked from "./views/Liked.svelte";

  let view: "grid" | "player" | "liked" = $state("grid");
  // Grid stays mounted (hidden) while the Player is open, so its filters survive; scroll and focus are restored by hand.
  let scrollY = 0;
  let opener: HTMLElement | null = null;

  async function open(next: "player" | "liked") {
    scrollY = window.scrollY;
    opener = document.activeElement instanceof HTMLElement && document.activeElement !== document.body ? document.activeElement : null;
    view = next;
    await tick();
    document.querySelector<HTMLElement>(`#${next}-view h1`)?.focus({ preventScroll: true });
  }
  const openPlayer = () => open("player");
  const openLiked = () => open("liked");

  async function closeToGrid() {
    view = "grid";
    await tick();
    window.scrollTo(0, scrollY);
    if (opener?.isConnected) opener.focus({ preventScroll: true });
    else document.querySelector<HTMLElement>("#grid-view .tile, #grid-view .chip")?.focus({ preventScroll: true });
  }
</script>

<div id="grid-view" class="view" hidden={view !== "grid"}>
  <Grid onopenplayer={openPlayer} onopenliked={openLiked} />
</div>
{#if view === "player"}
  <div id="player-view" class="view">
    <Player onback={closeToGrid} />
  </div>
{/if}
{#if view === "liked"}
  <div id="liked-view" class="view">
    <Liked onback={closeToGrid} onopenplayer={openPlayer} />
  </div>
{/if}

<style>
  .view { animation: view-in var(--motion-duration-base) var(--motion-easing) backwards; } /* not `both`: a retained transform would trap the fixed NowPlayingBar */
  .view[hidden] { display: none; }
  @keyframes view-in { from { opacity: 0; transform: translateY(8px); } }
  @media (prefers-reduced-motion: reduce) {
    .view { animation: none; }
  }
</style>
