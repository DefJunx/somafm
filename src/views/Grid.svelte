<script lang="ts">
  import { onMount } from "svelte";
  import type { Station } from "../lib/api";
  import { favouritesIn, filterByGenre, genresOf } from "../lib/genres";
  import { playerStore, statusLabel } from "../lib/player.svelte";
  import { settingsStore } from "../lib/settings.svelte";
  import { stationsStore } from "../lib/stations.svelte";
  import NowPlayingBar from "./NowPlayingBar.svelte";
  import Icon from "../lib/Icon.svelte";
  import StatusBadge from "../lib/StatusBadge.svelte";

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

  const SKELETON_COUNT = 12;

  const activeId = $derived(player.state.stationId);
  const activeKind = $derived(player.state.status.kind);
</script>

<main aria-busy={!store.ready}>
  {#if !store.ready}
    <!-- waiting for the first snapshot: Skeleton tiles, never the empty state -->
    <ul class="grid" aria-hidden="true">
      {#each Array(SKELETON_COUNT) as _, i (i)}
        <li class="skeleton"><span class="art"></span><span class="line"></span><span class="line short"></span></li>
      {/each}
    </ul>
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
          </button>
          <div class="meta">
            <div class="text">
              <span class="name" title={s.title}>{s.title}</span>
              <span class="genre">{s.genres.join(", ")}</span>
            </div>
            {#if s.id === activeId}
              <StatusBadge kind={activeKind} label={statusLabel(player.state)} data-testid="badge" />
            {/if}
          </div>
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
    grid-template-columns: repeat(auto-fill, minmax(min(140px, 100%), 1fr));
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
    border-radius: var(--radius-control);
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
    font-size: var(--font-size-6); color: var(--color-primary);
    background: rgb(0 0 0 / 0.45); opacity: 0; transition: opacity var(--motion-duration-fast) var(--motion-easing);
  }
  :global(.play > svg) { font-size: 2em; }
  .tile.active { outline: 3px solid var(--color-accent); outline-offset: -3px; }
  .tile:hover .play, .tile:focus-visible .play { opacity: 1; }
  .meta { display: flex; align-items: center; gap: var(--size-2); margin-block-start: var(--size-2); }
  .text { display: grid; flex: 1; min-width: 0; }
  .name, .genre { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .name { font-family: var(--font-heading); font-weight: var(--font-weight-6); color: var(--text-primary); }
  .genre { font-size: var(--font-size-0); color: var(--text-muted); }
  .skeleton { display: grid; gap: var(--size-2); }
  .skeleton .art, .skeleton .line {
    display: block; border-radius: var(--radius-control);
    background: linear-gradient(90deg, var(--surface-raised) 25%, var(--surface-control-strong) 50%, var(--surface-raised) 75%) 0 0 / 200% 100%;
    animation: shimmer 1.1s ease-in-out infinite;
  }
  .skeleton { animation: skeleton-in 0.4s var(--motion-easing) both, breathe 1.8s ease-in-out infinite; }
  .skeleton:nth-child(3n + 2) { animation-delay: 0.06s, 0.25s; }
  .skeleton:nth-child(3n) { animation-delay: 0.12s, 0.5s; }
  .skeleton .art { aspect-ratio: 1; }
  .skeleton .line { height: 1em; }
  .skeleton .short { width: 60%; }
  @keyframes shimmer { to { background-position: -200% 0; } }
  @keyframes breathe { 50% { opacity: 0.6; } }
  @keyframes skeleton-in { from { opacity: 0; transform: translateY(6px); } }
  @media (prefers-reduced-motion: reduce) {
    .skeleton, .skeleton .art, .skeleton .line { animation: none; }
  }
</style>
