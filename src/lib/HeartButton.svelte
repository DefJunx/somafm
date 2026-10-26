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
    flex: none; width: 32px; height: 32px; min-height: 32px; padding: 0;
    border-color: transparent; color: var(--ink-dim);
  }
  .heart:not(:disabled):hover { color: var(--ink); border-color: var(--hairline); }
  .heart.liked, .heart.liked:not(:disabled):hover { color: var(--sodium); border-color: transparent; }
</style>
