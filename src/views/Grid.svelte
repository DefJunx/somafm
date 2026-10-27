<script lang="ts">
  import { onMount } from "svelte";
  import type { Station } from "../lib/api";
  import { favouritesIn, filterByGenre, genresOf, hueOfGenre, hueVar, stationHueVar } from "../lib/genres";
  import { playerStore, statusLabel } from "../lib/player.svelte";
  import { settingsStore } from "../lib/settings.svelte";
  import { likedStore } from "../lib/liked.svelte";
  import { stationsStore } from "../lib/stations.svelte";
  import NowPlayingBar from "./NowPlayingBar.svelte";
  import TopBar from "../lib/TopBar.svelte";
  import StatusBadge from "../lib/StatusBadge.svelte";

  let { onopenplayer, onopenliked, store = stationsStore, player = playerStore, settings = settingsStore, liked = likedStore }: {
    onopenplayer?: () => void;
    onopenliked?: () => void;
    store?: typeof stationsStore;
    player?: typeof playerStore;
    settings?: typeof settingsStore;
    liked?: typeof likedStore;
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
    liked.init().catch(() => {});
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

<TopBar current="stations" onliked={onopenliked} {settings} />
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
    <h1 class="sr-only">Stations</h1>
    <div class="chips" role="group" aria-label="Filter stations">
      <button class="chip" aria-pressed={selected === null && !settings.favouritesOnly} onclick={() => { genre = null; settings.setFavouritesOnly(false).catch(() => {}); }}>All</button>
      <button class="chip" aria-pressed={settings.favouritesOnly} onclick={() => settings.setFavouritesOnly(!settings.favouritesOnly).catch(() => {})}>Favourites</button>
      {#each genres as g (g)}
        <button class="chip" style:--hue={hueVar(hueOfGenre(g))} aria-pressed={selected === g} onclick={() => (genre = g)}>{g}</button>
      {/each}
    </div>
    <ul class="grid">
      {#each visible as s (s.id)}
        <li style:--hue={stationHueVar(s)} class:active={s.id === activeId}>
          <button class="tile" aria-current={s.id === activeId ? "true" : undefined} onclick={() => onPlay(s)} aria-label={`Play ${s.title}`}>
            {#if failed.has(s.largeImage)}
              <span class="fallback">{s.title}</span>
            {:else}
              <img src={s.largeImage} alt={s.title} loading="lazy" onerror={() => (failed = new Set(failed).add(s.largeImage))} />
            {/if}
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
<NowPlayingBar {onopenplayer} {player} stations={store} {liked} />

<style>
  main { padding: 24px; padding-block-end: calc(var(--bar-h) + 24px); }
  .empty { display: grid; justify-items: center; gap: 16px; margin-block: 80px; }
  .empty h1 { font-size: var(--text-xl); }
  .chips { display: flex; flex-wrap: wrap; gap: 8px; margin-block-end: 24px; }
  .chip { --hue: var(--ink); --hue-ring: var(--hue); position: relative; font-weight: 400; }
  .chip[aria-pressed="true"] { font-weight: 700; }
  .chip[aria-pressed="true"]::after { content: ""; position: absolute; left: 8px; right: 8px; bottom: 3px; height: 2px; background: var(--hue); }
  .grid {
    list-style: none; margin: 0; padding: 0;
    display: grid; grid-template-columns: repeat(auto-fill, minmax(min(160px, 100%), 1fr)); gap: 24px 16px;
  }
  .grid > li { position: relative; min-width: 0; --hue-ring: var(--hue); }
  .grid > li:not(.skeleton)::before {
    content: ""; display: block; width: 24px; height: 2px; margin-block-end: 8px; background: var(--hue);
    transition: width var(--motion-duration-base) var(--motion-easing);
  }
  .grid > li.active::before { width: 100%; }
  .tile {
    position: relative; display: block; width: 100%; height: auto; aspect-ratio: 1; padding: 0; overflow: hidden;
    background: var(--well); border: 1px solid var(--hairline); border-radius: var(--radius);
  }
  .tile:not(:disabled):hover { border-color: var(--hue); }
  .active .tile { border-color: var(--ink); }
  .tile img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .fallback { display: grid; place-items: center; height: 100%; padding: 8px; text-align: center; font-weight: 700; font-size: var(--text-md); line-height: 24px; overflow-wrap: anywhere; }
  .meta { display: flex; flex-direction: column; align-items: flex-start; gap: 4px; margin-block-start: 8px; }
  .text { display: grid; min-width: 0; max-width: 100%; }
  .name, .genre { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .name { font-size: var(--text-md); line-height: 24px; font-weight: 700; color: var(--ink); }
  .genre { color: var(--ink-dim); letter-spacing: var(--track-body); }
  .skeleton { display: grid; gap: 8px; }
    .skeleton .art, .skeleton .line {
    display: block; background: var(--plate); border-radius: var(--radius);
    animation: shimmer 1.4s ease-in-out infinite;
  }
  .skeleton .art { aspect-ratio: 1; border: 1px solid var(--hairline); }
  .skeleton .line { height: 16px; }
  .skeleton .short { width: 60%; }
  @keyframes shimmer { 50% { opacity: 0.5; } }
  @media (prefers-reduced-motion: reduce) {
    .skeleton .art, .skeleton .line { animation: none; }
  }
  @media (max-width: 720px) {
    main { padding: 16px; padding-block-end: calc(var(--bar-h) + 16px); }
  }
</style>
