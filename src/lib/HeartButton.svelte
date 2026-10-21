<script lang="ts">
  import Icon from "./Icon.svelte";
  import { rawText } from "./tracks";
  import type { likedStore } from "./liked.svelte";

  /** Heart for one track; `raw` is the like key (Station id + raw), shown as its display text in the accessible name. */
  let { store, stationId, raw }: { store: typeof likedStore; stationId: string; raw: string } = $props();
  const liked = $derived(store.isLiked(stationId, raw));
</script>

<button class="heart" class:liked aria-pressed={liked} aria-label={`${liked ? "Unlike" : "Like"} ${rawText(raw)}`} title={liked ? "Remove from Liked" : "Add to Liked"} onclick={() => store.toggle(stationId, raw).catch(() => {})}>
  <Icon name={liked ? "heart-filled" : "heart"} />
</button>

<style>
  .heart {
    flex-shrink: 0; display: flex; align-items: center; justify-content: center;
    padding: var(--size-2); background: none; border: none; cursor: pointer;
    color: var(--text-muted);
    transition: color var(--motion-duration-fast) var(--motion-easing);
  }
  .heart:hover { color: var(--color-accent); }
  .heart.liked { color: var(--color-primary); }
  .heart:focus-visible { outline: var(--focus-ring); outline-offset: 2px; }
</style>
