<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { slide } from "svelte/transition";
  import { openUrl, type LikedTrack } from "../lib/api";
  import { likedStore } from "../lib/liked.svelte";
  import { playerStore } from "../lib/player.svelte";
  import { stationsStore } from "../lib/stations.svelte";
  import { rawText, searchLinks, splitRaw } from "../lib/tracks";
  import { stationHueVar } from "../lib/genres";
  import { filterLiked, groupLiked } from "../lib/likedGroups";
  import Icon from "../lib/Icon.svelte";
  import NowPlayingBar from "./NowPlayingBar.svelte";
  import TopBar from "../lib/TopBar.svelte";

  let { onback, onopenplayer, player = playerStore, stations = stationsStore, liked = likedStore, now = Date.now }: {
    onback?: () => void;
    onopenplayer?: () => void;
    player?: typeof playerStore;
    stations?: typeof stationsStore;
    liked?: typeof likedStore;
    /** Clock, injectable for tests. */
    now?: () => number;
  } = $props();

  onMount(() => {
    player.init().catch(() => {});
    stations.init().catch(() => {});
    liked.init().catch(() => {});
  });

  const TOAST_MS = 5000;
  const COPIED_MS = 1500;
  const keyOf = (t: LikedTrack) => `${t.stationId}\n${t.raw}`;
  const station = (id: string) => stations.stations.find((s) => s.id === id);
  const timeFormat = new Intl.DateTimeFormat(undefined, { hour: "2-digit", minute: "2-digit" });
  const dayFormat = new Intl.DateTimeFormat(undefined, { day: "numeric", month: "short" });
  const reducedMotion = () => typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
  const collapse = (node: Element) => slide(node, { duration: reducedMotion() ? 0 : 200 });

  let query = $state("");
  let headerEl: HTMLElement | undefined = $state();
  let headerHeight = $state(0);
  $effect(() => {
    if (!headerEl || typeof ResizeObserver === "undefined") return;
    const el = headerEl;
    const ro = new ResizeObserver(() => (headerHeight = el.offsetHeight));
    ro.observe(el);
    return () => ro.disconnect();
  });
  let hidden: Set<string> = $state(new Set());
  let failed: Set<string> = $state(new Set());
  let copiedKey: string | null = $state(null);
  let toast: LikedTrack | null = $state(null);
  let copyTimer: ReturnType<typeof setTimeout> | undefined;
  let toastTimer: ReturnType<typeof setTimeout> | undefined;
  onDestroy(() => { clearTimeout(copyTimer); clearTimeout(toastTimer); });

  const visible = $derived(liked.tracks.filter((t) => !hidden.has(keyOf(t))));
  const filtered = $derived(filterLiked(visible, query, (id) => station(id)?.title));
  const groups = $derived(groupLiked(filtered, now()));

  function unhide(t: LikedTrack) {
    const next = new Set(hidden);
    next.delete(keyOf(t));
    hidden = next;
  }
  function remove(t: LikedTrack) {
    hidden = new Set(hidden).add(keyOf(t));
    toast = t;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = null), TOAST_MS);
    liked.remove(t.stationId, t.raw).catch(() => { unhide(t); if (toast === t) toast = null; });
  }
  async function undo() {
    const t = toast;
    if (!t) return;
    toast = null;
    clearTimeout(toastTimer);
    try { await liked.restore(t); } catch { /* the row stays removed */ return; }
    unhide(t);
  }
  // A removed row stays hidden only until the store confirms; forget keys the store no longer holds.
  $effect(() => {
    const keys = new Set(liked.tracks.map(keyOf));
    if ([...hidden].some((k) => !keys.has(k))) hidden = new Set([...hidden].filter((k) => keys.has(k)));
  });

  async function copy(t: LikedTrack) {
    try { await navigator.clipboard?.writeText(t.raw); } catch { return; }
    copiedKey = keyOf(t);
    clearTimeout(copyTimer);
    copyTimer = setTimeout(() => (copiedKey = null), COPIED_MS);
  }
  const play = (id: string) => player.play(id).catch(() => {});
  const initials = (title: string) => title.split(/\s+/).filter(Boolean).slice(0, 2).map((w) => w[0]).join("").toUpperCase();
</script>

<TopBar current="liked" backLabel onstations={onback} />
<main>
  <header bind:this={headerEl}>
    <h1 tabindex="-1">Liked</h1>
    {#if liked.tracks.length > 0}
      <input
        class="filter" type="search" placeholder="Filter liked tracks" aria-label="Filter liked tracks"
        bind:value={query}
        onkeydown={(e) => { if (e.key === "Escape" && query) { query = ""; e.preventDefault(); } }}
      />
    {/if}
  </header>

  {#if liked.tracks.length === 0}
    <div class="empty">
      <p class="empty-title">No liked tracks yet</p>
      <p class="muted">Tap the heart next to a playing track to save it here.</p>
      <button class="solid" onclick={() => onback?.()}>Browse stations</button>
    </div>
  {:else if groups.length === 0}
    <div class="empty">
      <p class="muted">No liked tracks match “{query.trim()}”</p>
      <button class="solid" onclick={() => (query = "")}>Clear filter</button>
    </div>
  {:else}
    <div style:--head-h="{headerHeight}px">
      {#each groups as g (g.key)}
        <section>
          <h2>{g.label}</h2>
          <ul>
            {#each g.items as t (keyOf(t))}
              {@const s = station(t.stationId)}
              {@const { artist, title } = splitRaw(t.raw)}
              {@const text = rawText(t.raw)}
              <li out:collapse style:--hue={stationHueVar(s)}>
                <div class="thumb">
                  {#if s}
                    <button class="art" aria-label={`Play ${s.title}`} onclick={() => play(t.stationId)}>
                      {#if failed.has(s.largeImage)}
                        <span class="fallback" aria-hidden="true">{initials(s.title)}</span>
                      {:else}
                        <img src={s.largeImage} alt="" loading="lazy" onerror={() => (failed = new Set(failed).add(s.largeImage))} />
                      {/if}
                      <span class="overlay" aria-hidden="true"><Icon name="play" /></span>
                    </button>
                  {:else}
                    <span class="art"><span class="fallback" aria-hidden="true">{initials(t.stationId)}</span></span>
                  {/if}
                </div>
                <div class="text">
                  <span class="title" title={text}>{title}</span>
                  {#if artist}<span class="artist" title={text}>{artist}</span>{/if}
                </div>
                <div class="meta">
                  {#if s}
                    <button class="station" aria-label={`Play ${s.title}`} onclick={() => play(t.stationId)}>{s.title}</button>
                  {:else}
                    <span class="station missing">{t.stationId}</span>
                  {/if}
                  <span class="mark" aria-hidden="true"><Icon name="heart-filled" /></span>
                  <time datetime={new Date(t.likedAt).toISOString()}>{g.timeOnly ? timeFormat.format(t.likedAt) : dayFormat.format(t.likedAt)}</time>
                </div>
                <div class="actions">
                  <button class="icon" aria-label={`${copiedKey === keyOf(t) ? "Copied" : "Copy"} ${text}`} title={copiedKey === keyOf(t) ? "Copied" : "Copy"} onclick={() => copy(t)}>
                    <Icon name={copiedKey === keyOf(t) ? "check" : "copy"} />
                  </button>
                  <span class="search" role="group" aria-label="Search on">
                    {#each searchLinks(t.raw) as link (link.name)}
                      <button class="text" onclick={() => openUrl(link.url).catch(() => {})}>{link.name}</button>
                    {/each}
                  </span>
                  <button class="icon" aria-label={`Remove ${text}`} title="Remove" onclick={() => remove(t)}>
                    <Icon name="close" />
                  </button>
                </div>
              </li>
            {/each}
          </ul>
        </section>
      {/each}
    </div>
  {/if}
</main>

<span class="sr-only" aria-live="polite">{copiedKey ? "Copied" : ""}</span>
{#if toast}
  <div class="toast" role="status" aria-live="polite">
    <span>Removed {splitRaw(toast.raw).title}</span>
    <button onclick={undo}>Undo</button>
  </div>
{/if}
<NowPlayingBar {onopenplayer} {player} {stations} {liked} />

<style>
  main { max-width: 880px; padding: 0 24px; padding-block-end: calc(var(--bar-h) * 2); min-width: 0; }
  header {
    position: sticky; top: 0; z-index: 3; background: var(--ground);
    display: flex; flex-wrap: wrap; align-items: center; gap: 8px 24px;
    padding: 16px 8px 16px 0; border-block-end: 1px solid var(--hairline);
  }
  h1 { flex: 1 1 auto; }
  h1:focus { outline: none; }
  .filter { flex: 0 1 288px; min-width: 0; }
  .muted { color: var(--ink-dim); }
  .empty { display: grid; justify-items: center; gap: 16px; margin-block: 80px; text-align: center; }
  .empty-title { font-size: var(--text-md); line-height: 24px; font-weight: 700; }

  h2 {
    position: sticky; top: var(--head-h, 0); z-index: 2; padding-block: 16px 8px;
    background: var(--ground); border-block-end: 1px solid var(--hairline-soft);
  }
  ul { list-style: none; margin: 0 0 16px; padding: 0; }
  li {
    position: relative; display: grid; grid-template-columns: 32px minmax(0, 1fr) auto;
    grid-template-areas: "thumb text meta"; align-items: center; gap: 16px;
    min-height: 48px; padding: 8px 8px 8px 16px; border-block-end: 1px solid var(--hairline-soft);
  }
  li::before { content: ""; position: absolute; left: 0; top: 8px; bottom: 8px; width: 1px; background: var(--hue); }
  li:hover, li:focus-within { background: var(--plate); }
  .thumb { grid-area: thumb; }
  .art {
    position: relative; display: block; width: 32px; height: 32px; box-sizing: border-box; padding: 0; overflow: hidden;
    background: var(--well); border: 1px solid var(--hairline); border-radius: 0;
  }
  .art::after { content: ""; position: absolute; left: 0; right: 0; bottom: 0; height: 2px; background: var(--hue); }
  button.art { cursor: pointer; }
  .art img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .fallback { display: grid; place-items: center; height: 100%; font-size: var(--text-sm); color: var(--ink-dim); }
  .overlay {
    position: absolute; inset: 0; display: grid; place-items: center; color: var(--ink);
    background: var(--scrim); opacity: 0; transition: opacity var(--motion-duration-fast) var(--motion-easing);
  }
  li:hover .overlay, .art:focus-visible .overlay { opacity: 1; }
  .art { --hue-ring: var(--hue); }

  .text { grid-area: text; display: grid; min-width: 0; }
  .title, .artist { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .title { color: var(--ink); font-size: var(--text-md); line-height: 24px; font-weight: 500; }
  .artist { color: var(--ink-dim); letter-spacing: var(--track-body); }

  .meta { grid-area: meta; display: flex; align-items: center; gap: 16px; color: var(--ink-dim); min-width: 0; }
  .mark { color: var(--sodium); display: flex; }
  .station { height: auto; min-height: 0; padding: 0; border: none; color: var(--ink-dim); max-width: 12rem; overflow: hidden; display: block; white-space: nowrap; text-overflow: ellipsis; }
  button.station:not(:disabled):hover { color: var(--ink); border: none; }
  time { min-width: 48px; text-align: end; }

  .actions {
    position: absolute; inset-block: 0; inset-inline-end: 0; display: flex; align-items: center; gap: 8px;
    padding-inline: 8px; background: var(--plate);
    opacity: 0; pointer-events: none; transition: opacity var(--motion-duration-fast) var(--motion-easing);
  }
  li:hover .actions, li:focus-within .actions { opacity: 1; pointer-events: auto; }
  .search { display: flex; align-items: center; gap: 4px; }
  .actions button { color: var(--ink-dim); border-color: transparent; }
  .actions button:not(:disabled):hover { color: var(--ink); border-color: var(--hairline); }
  .actions .icon { width: 32px; padding: 0; }
  .actions .text { padding: 0 8px; }

  @media (hover: none), (max-width: 36rem) {
    li { grid-template-areas: "thumb text meta" ". actions actions"; row-gap: 4px; }
    .actions { position: static; grid-area: actions; margin-inline-start: -8px; opacity: 1; pointer-events: auto; background: none; padding: 0; flex-wrap: wrap; }
    li:hover, li:focus-within { background: none; }
    .station { max-width: 8rem; }
    main { padding: 0 16px; padding-block-end: calc(var(--bar-h) * 2); }
  }

  .toast {
    position: fixed; z-index: 5; inset-inline: 16px; inset-block-end: calc(var(--bar-h) + 16px);
    margin-inline: auto; width: fit-content; max-width: calc(100% - 32px);
    display: flex; align-items: center; gap: 16px; padding: 8px 8px 8px 16px;
    background: var(--plate); border: 1px solid var(--hairline); border-radius: var(--radius);
  }
  .toast span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
