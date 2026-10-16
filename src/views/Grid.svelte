<script lang="ts">
  import { onMount } from "svelte";
  import type { Station } from "../lib/api";
  import { filterByGenre, genresOf } from "../lib/genres";
  import { stationsStore } from "../lib/stations.svelte";

  let { onopenplayer, store = stationsStore }: { onopenplayer?: () => void; store?: typeof stationsStore } = $props();

  let genre: string | null = $state(null);
  let failed: Set<string> = $state(new Set());

  const genres = $derived(genresOf(store.stations));
  // A genre that vanished from the feed falls back to All.
  const selected = $derived(genre !== null && !genres.includes(genre) ? null : genre);

  onMount(() => {
    store.init().catch(() => {}); // a later mount retries
  });

  // Playback arrives in issue 03; for now a Tile click does nothing.
  function onPlay(_station: Station) {}
</script>

<main>
  {#if !store.ready}
    <!-- waiting for the first snapshot: render nothing, no flash -->
  {:else if !store.loaded || store.stations.length === 0}
    <section class="empty">
      <h1>Can't reach SomaFM</h1>
      <button onclick={() => store.retry()} disabled={store.retrying}>Retry</button>
    </section>
  {:else}
    <div class="chips" role="group" aria-label="Filter by genre">
      <button class="chip" aria-pressed={selected === null} onclick={() => (genre = null)}>All</button>
      {#each genres as g (g)}
        <button class="chip" aria-pressed={selected === g} onclick={() => (genre = g)}>{g}</button>
      {/each}
    </div>
    <ul class="grid">
      {#each filterByGenre(store.stations, selected) as s (s.id)}
        <li>
          <button class="tile" onclick={() => onPlay(s)} aria-label={`Play ${s.title}`}>
            {#if failed.has(s.largeImage)}
              <span class="fallback">{s.title}</span>
            {:else}
              <img src={s.largeImage} alt={s.title} loading="lazy" onerror={() => (failed = new Set(failed).add(s.largeImage))} />
            {/if}
            <span class="play" aria-hidden="true">&#9654;</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</main>

<style>
  main { padding: var(--size-4); }
  .empty { display: grid; justify-items: center; gap: var(--size-3); margin-block: var(--size-10); }
  .chips { display: flex; flex-wrap: wrap; gap: var(--size-2); margin-block-end: var(--size-4); }
  .chip {
    border-radius: var(--radius-round);
    background: var(--surface-3);
    padding-inline: var(--size-3);
  }
  .chip[aria-pressed="true"] { background: var(--indigo-7); color: white; }
  .grid {
    list-style: none;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: var(--size-3);
  }
  .tile {
    position: relative;
    display: block;
    width: 100%;
    aspect-ratio: 1;
    padding: 0;
    overflow: hidden;
    border: none;
    border-radius: var(--radius-2);
    background: var(--surface-2);
    cursor: pointer;
  }
  .tile img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .fallback { display: grid; place-items: center; height: 100%; padding: var(--size-2); text-align: center; font-weight: var(--font-weight-6); }
  .play {
    position: absolute; inset: 0; display: grid; place-items: center;
    font-size: var(--font-size-6); color: white;
    background: rgb(0 0 0 / 0.45); opacity: 0; transition: opacity 0.15s;
  }
  .tile:hover .play, .tile:focus-visible .play { opacity: 1; }
</style>
