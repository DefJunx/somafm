<script lang="ts">
  import { onMount } from "svelte";
  import type { Station } from "../lib/api";
  import { favouritesIn, filterByGenre, genresOf } from "../lib/genres";
  import { playerStore } from "../lib/player.svelte";
  import { settingsStore } from "../lib/settings.svelte";
  import { stationsStore } from "../lib/stations.svelte";
  import NowPlayingBar from "./NowPlayingBar.svelte";
  import Icon from "../lib/Icon.svelte";

  let { onopenplayer, store = stationsStore, player = playerStore, settings = settingsStore }: {
    onopenplayer?: () => void;
    store?: typeof stationsStore;
    player?: typeof playerStore;
    settings?: typeof settingsStore;
  } = $props();

  let genre: string | null = $state(null);
  let failed: Set<string> = $state(new Set());

  const genres = $derived(genresOf(store.stations));
  // A genre that vanished from the feed falls back to All.
  const selected = $derived(genre !== null && !genres.includes(genre) ? null : genre);

  onMount(() => {
    store.init().catch(() => {}); // a later mount retries
    player.init().catch(() => {});
    settings.init().catch(() => {});
  });

  const visible = $derived.by(() => {
    const byGenre = filterByGenre(store.stations, selected);
    return settings.favouritesOnly ? favouritesIn(byGenre, settings.favourites) : byGenre;
  });

  function onPlay(station: Station) {
    player.play(station.id).catch(() => {});
    onopenplayer?.();
  }

  const activeId = $derived(player.state.stationId);
  const activeKind = $derived(player.state.status.kind);
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
    <div class="chips" role="group" aria-label="Filter stations">
      <button class="chip" aria-pressed={selected === null && !settings.favouritesOnly} onclick={() => { genre = null; settings.setFavouritesOnly(false).catch(() => {}); }}>All</button>
      <button class="chip" aria-pressed={settings.favouritesOnly} onclick={() => settings.setFavouritesOnly(!settings.favouritesOnly).catch(() => {})}>Favourites</button>
      {#each genres as g (g)}
        <button class="chip" aria-pressed={selected === g} onclick={() => (genre = g)}>{g}</button>
      {/each}
    </div>
    <ul class="grid">
      {#each visible as s (s.id)}
        <li>
          <button class="tile" class:active={s.id === activeId} aria-current={s.id === activeId ? "true" : undefined} onclick={() => onPlay(s)} aria-label={`Play ${s.title}`}>
            {#if failed.has(s.largeImage)}
              <span class="fallback">{s.title}</span>
            {:else}
              <img src={s.largeImage} alt={s.title} loading="lazy" onerror={() => (failed = new Set(failed).add(s.largeImage))} />
            {/if}
            <span class="play"><Icon name="play" /></span>
            {#if s.id === activeId}
              <span class="badge" data-testid="badge" data-status={activeKind} title={activeKind}>
                {#if activeKind === "error"}<Icon name="warning" />{:else if activeKind === "connecting"}<i class="dot"></i>{:else if activeKind === "playing"}<i class="eq"><b></b><b></b><b></b></i>{:else}<i class="dot still"></i>{/if}
              </span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</main>
<NowPlayingBar {onopenplayer} {player} stations={store} />

<style>
  main { padding: var(--size-4); padding-block-end: var(--size-10); }
  .empty { display: grid; justify-items: center; gap: var(--size-3); margin-block: var(--size-10); }
  .empty button {
    padding: var(--size-2) var(--size-4);
  }
  .chips { display: flex; flex-wrap: wrap; gap: var(--size-2); margin-block-end: var(--size-4); }
  .chip {
    border-radius: var(--radius-round);
    background: var(--surface-control);
    padding-inline: var(--size-3);
    color: var(--text-primary);
    padding-block: var(--size-2);
    border: none;
    cursor: pointer;
    transition: background-color var(--motion-duration-fast) var(--motion-easing);
  }
  .chip:not(:disabled):hover {
    background-color: var(--color-accent);
  }
  .chip:focus-visible {
    outline: var(--focus-ring);
    outline-offset: 2px;
  }
  .chip:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  .chip[aria-pressed="true"] { background: var(--surface-control-strong); color: var(--text-primary); }
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
    background: var(--surface-raised);
    cursor: pointer;
  }
  .tile:focus-visible {
    outline: var(--focus-ring);
    outline-offset: 2px;
  }
  .tile img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .fallback { display: grid; place-items: center; height: 100%; padding: var(--size-2); text-align: center; font-weight: var(--font-weight-6); font-family: var(--font-heading); }
  .play {
    position: absolute; inset: 0; display: grid; place-items: center;
    font-size: var(--font-size-6); color: white;
    background: rgb(0 0 0 / 0.45); opacity: 0; transition: opacity var(--motion-duration-fast) var(--motion-easing);
  }
  :global(.play > svg) { font-size: 2em; }
  .tile.active { outline: 3px solid var(--color-accent); outline-offset: -3px; }
  .badge {
    position: absolute; inset-block-end: var(--size-2); inset-inline-end: var(--size-2);
    padding: var(--size-1) var(--size-2); border-radius: var(--radius-round);
    background: rgb(0 0 0 / 0.7); color: white; line-height: 1;
  }
  .dot { display: inline-block; width: 0.6em; height: 0.6em; border-radius: 50%; background: currentColor; animation: pulse 1s ease-in-out infinite; }
  .eq { display: inline-flex; align-items: flex-end; gap: 2px; height: 1em; }
  .eq b { width: 3px; background: currentColor; animation: eq 0.9s ease-in-out infinite; }
  .eq b:nth-child(2) { animation-delay: 0.2s; }
  .eq b:nth-child(3) { animation-delay: 0.4s; }
  .dot.still { animation: none; }
  @keyframes pulse { 50% { opacity: 0.25; } }
  @keyframes eq { 0%, 100% { height: 20%; } 50% { height: 100%; } }
  .tile:hover .play, .tile:focus-visible .play { opacity: 1; }
</style>
