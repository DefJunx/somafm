<script lang="ts">
  import { onMount } from "svelte";
  import { nowPlayingText, playerStore, primaryAction, runPrimaryAction, statusLabel, statusText } from "../lib/player.svelte";
  import { settingsStore } from "../lib/settings.svelte";
  import { historyStore } from "../lib/history.svelte";
  import { likedStore } from "../lib/liked.svelte";
  import { trackText } from "../lib/tracks";
  import { stationsStore } from "../lib/stations.svelte";
  import { slide } from "svelte/transition";
  import type { Station } from "../lib/api";
  import { stationHueVar } from "../lib/genres";
  import Icon from "../lib/Icon.svelte";
  import StationRail from "../lib/StationRail.svelte";
  import TopBar from "../lib/TopBar.svelte";
  import HeartButton from "../lib/HeartButton.svelte";
  import StatusBadge from "../lib/StatusBadge.svelte";
  import Visualizer from "../lib/Visualizer.svelte";
  import { setVisualizerActive } from "../lib/api";

  let { onback, onopenliked, player = playerStore, stations = stationsStore, settings = settingsStore, history = historyStore, liked = likedStore }: {
    onback?: () => void;
    onopenliked?: () => void;
    player?: typeof playerStore;
    stations?: typeof stationsStore;
    settings?: typeof settingsStore;
    history?: typeof historyStore;
    liked?: typeof likedStore;
  } = $props();

  let lastVolume = 0.8;
  let imageFailed = $state(false);

  const s = $derived(player.state);
  const station = $derived(stations.stations.find((x) => x.id === s.stationId));
  const title = $derived(station?.title ?? s.stationId ?? "");
  const muted = $derived(s.volume === 0);
  const action = $derived(primaryAction(s));
  const hue = $derived(stationHueVar(station));
  const live = $derived(s.status.kind === "playing" || s.status.kind === "connecting");
  const reconnecting = $derived(s.status.kind === "connecting" && s.status.reconnecting);
  const selectStation = (next: Station) => { if (next.id !== s.stationId) player.play(next.id).catch(() => {}); };

  onMount(() => {
    const onVisibility = () => (visible = !document.hidden);
    document.addEventListener("visibilitychange", onVisibility);
    player.init().catch(() => {});
    stations.init().catch(() => {});
    settings.init().catch(() => {});
    liked.init().catch(() => {});
    return () => document.removeEventListener("visibilitychange", onVisibility);
  });

  // The Player screen is only mounted while visible: fetch on open, then when a new track starts. A track change
  // while the window is hidden is fetched once on return, never on a timer.
  const nowRaw = $derived(s.nowPlaying?.raw ?? null);
  const activeId = $derived(s.stationId);
  let loadedKey = "";
  let visible = $state(!document.hidden);
  const fetchHistory = (id: string, raw: string | null) => {
    loadedKey = `${id}\n${raw ?? ""}`;
    history.load(id);
  };
  $effect(() => {
    const id = activeId;
    const raw = nowRaw;
    if (!id || !visible) return;
    // A null Now playing (stop, reconnect) is not a new track: keep what we have unless the Station changed.
    if (raw === null && loadedKey.startsWith(`${id}\n`)) return;
    if (`${id}\n${raw ?? ""}` !== loadedKey) fetchHistory(id, raw);
  });
  // Rust emits `spectrum` only while active: on while this screen is visible, off on hide and unmount (back to the Grid).
  $effect(() => {
    if (!visible) return;
    setVisualizerActive(true).catch(() => {});
    return () => { setVisualizerActive(false).catch(() => {}); };
  });
  const reducedMotion = () => typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
  // History rows keep their identity across refetches, so a new track slides in at the top and the rest settle down in place.
  const rows = $derived.by(() => {
    const seen = new Map<string, number>();
    return history.entries.map((e) => {
      const base = `${e.time}\n${e.raw}`;
      const n = seen.get(base) ?? 0;
      seen.set(base, n + 1);
      return { e, key: n ? `${base}\n${n}` : base };
    });
  });
  const timeFormat = new Intl.DateTimeFormat(undefined, { hour: "2-digit", minute: "2-digit" });

  const favourite = $derived(s.stationId !== null && settings.favourites.includes(s.stationId));

  function toggleMute() {
    if (muted) {
      player.setVolume(lastVolume || 0.8);
    } else {
      lastVolume = s.volume;
      player.setVolume(0);
    }
  }
</script>

<div class="screen" style:--hue={hue}>
  <TopBar current="player" backLabel onstations={onback} onliked={onopenliked} settings={settings} />
  <div class="body">
    <StationRail stations={stations.stations} activeId={s.stationId} kind={s.status.kind} {reconnecting} onselect={selectStation} />
    <main class="plate" class:live>
      <div class="head">
        <div class="art">
          {#if station && !imageFailed}
            <img src={station.xlImage || station.largeImage} alt={station.title} onerror={() => (imageFailed = true)} />
          {:else}
            <span class="fallback">{title}</span>
          {/if}
        </div>
        <section class="info">
          <h1 {title} tabindex="-1">{title}</h1>
          <div class="line">
            <p class="status" role="status"><StatusBadge kind={s.status.kind} label={statusLabel(s)} text={statusText(s) || statusLabel(s)} /></p>
            {#if station}
              <p class="meta tnum"><span>{station.genres.join(" / ")}</span><span class="sep" aria-hidden="true"></span><span>{station.listeners} listeners</span></p>
            {/if}
          </div>
          {#if station}
            <p class="desc">{station.description}</p>
          {/if}
          <div class="nowrow">
            <p class="now" title={nowPlayingText(s, title)}>{nowPlayingText(s, title)}</p>
            {#if s.stationId && s.nowPlaying}
              <HeartButton store={liked} stationId={s.stationId} raw={s.nowPlaying.raw} />
            {/if}
          </div>
          <div class="transport">
            <button class="main primary" onclick={() => runPrimaryAction(player)} disabled={action.busy} aria-busy={action.busy}>
              {#if action.busy}
                <span class="spinner" aria-hidden="true"></span>
              {/if}
              <span>{action.busy ? "Connecting" : action.label}</span>
            </button>
            <div class="volume">
              <button class="icon-btn" onclick={toggleMute} aria-pressed={muted} aria-label={muted ? "Unmute" : "Mute"}>
                <Icon name={muted ? "mute" : "volume"} />
              </button>
              <input
                type="range" min="0" max="1" step="0.01" aria-label="Volume"
                value={s.volume}
                style:--fill="{s.volume * 100}%"
                oninput={(e) => player.setVolume(e.currentTarget.valueAsNumber)}
              />
              <span class="vol-value tnum" aria-hidden="true">{Math.round(s.volume * 100)}</span>
            </div>
            {#if s.stationId}
              <button class="icon-btn fav" aria-pressed={favourite} aria-label={favourite ? "Remove from Favourites" : "Add to Favourites"} title={favourite ? "Remove from Favourites" : "Add to Favourites"} onclick={() => s.stationId && settings.toggleFavourite(s.stationId).catch(() => {})}>
                <Icon name={favourite ? "star-filled" : "star"} />
              </button>
            {/if}
          </div>
        </section>
      </div>
      {#if settings.showVisualizer}
        <div class="viz"><Visualizer active={visible} playing={s.status.kind === "playing"} {hue} /></div>
      {/if}
      {#if s.stationId}
        <section class="history" aria-label="Station history">
          {#if history.failed}
            <p class="unavailable">History unavailable</p>
          {:else if rows.length > 0}
            <div class="hh" aria-hidden="true"><span>Station history</span><span>Time</span></div>
            <ul aria-label="Station history">
              {#each rows as { e, key } (key)}
                {@const isLive = s.status.kind === "playing" && s.nowPlaying?.raw === e.raw && e === history.entries[0]}
                <li class:live={isLive} class:liked={liked.isLiked(s.stationId, e.raw)} in:slide={{ duration: reducedMotion() ? 0 : 280 }}>
                  <span class="track" title={trackText(e.artist, e.title)}>{trackText(e.artist, e.title)}</span>
                  <HeartButton store={liked} stationId={s.stationId} raw={e.raw} />
                  <time class="when" datetime={new Date(e.time * 1000).toISOString()}>{timeFormat.format(e.time * 1000)}</time>
                </li>
              {/each}
            </ul>
          {/if}
        </section>
      {/if}
    </main>
  </div>
</div>

<style>
  .screen { display: flex; flex-direction: column; min-height: 100dvh; min-width: 0; }
  .body { flex: 1; display: grid; grid-template-columns: var(--rail-w) minmax(0, 1fr); column-gap: var(--rail-gap); padding-inline-end: 24px; }
  .plate {
    position: relative; min-width: 0; margin-block: 16px; padding: 24px 24px 0 32px;
    background: var(--plate); border-radius: 0 var(--radius) var(--radius) 0;
  }
  .plate.live::before { content: ""; position: absolute; left: 0; top: 0; bottom: 0; width: 2px; background: var(--hue); filter: drop-shadow(0 0 4px color-mix(in srgb, var(--hue) 35%, transparent)); }
  .head { display: flex; flex-wrap: wrap; gap: 24px 32px; }
  .art {
    position: relative; flex: none; width: 216px; height: 216px; box-sizing: border-box; overflow: hidden;
    display: grid; place-items: center; background: var(--well); border: 1px solid var(--hairline);
  }
  .art img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .fallback { font-weight: 700; padding: 16px; text-align: center; font-size: var(--text-md); line-height: 24px; overflow-wrap: anywhere; }
  .info { flex: 1 1 260px; min-width: 0; display: flex; flex-direction: column; align-items: flex-start; }
  .info > * { min-width: 0; max-width: 100%; }
  .line { display: flex; flex-wrap: wrap; align-items: center; gap: 4px 8px; }
  .status { margin: 0; }
  h1:focus { outline: none; }
  h1 {
    margin-block: 0 8px; display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 2; line-clamp: 2;
    overflow: hidden; overflow-wrap: anywhere;
  }
  .meta { display: flex; align-items: center; gap: 8px; color: var(--ink-dim); letter-spacing: var(--track-body); flex-wrap: wrap; }
  .sep { width: 4px; height: 4px; background: var(--hairline); }
  .desc { margin-block: 16px 8px; font-size: var(--text-md); line-height: 24px; max-inline-size: 44ch; }
  .nowrow { display: flex; align-items: center; gap: 8px; width: 100%; min-height: 32px; }
  .now { flex: 1; min-width: 0; margin: 0; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; color: var(--ink); font-weight: 500; }
  .transport { display: flex; flex-wrap: nowrap; align-items: center; gap: 16px; width: 100%; margin-block-start: 8px; }
  .main { width: 96px; padding: 0; }
  .volume { flex: 1 1 auto; min-width: 96px; display: flex; align-items: center; gap: 8px; color: var(--ink-dim); }
  .volume button { flex: none; border-color: transparent; }
  .volume button:hover { color: var(--ink); }
  .vol-value { flex: none; width: 3ch; text-align: right; color: var(--ink-dim); }
  .fav[aria-pressed="true"] { color: var(--ink); }

  .viz { margin-block: 24px 16px; }
  .viz :global(canvas) { filter: drop-shadow(0 0 4px color-mix(in srgb, var(--hue) 35%, transparent)); }
  .history { padding-block-end: 16px; }
  .hh { display: flex; justify-content: space-between; align-items: center; height: 24px; border-block-end: 1px solid var(--hairline); color: var(--ink-dim); font-weight: 500; letter-spacing: var(--track-label); text-transform: uppercase; }
  .history ul { list-style: none; margin: 0; padding: 0; }
  .history li {
    position: relative; display: flex; align-items: center; gap: 8px; height: 32px; padding-inline: 16px 0;
    color: var(--ink-dim); border-block-end: 1px solid var(--hairline-soft); overflow: hidden;
  }
  .history li.live { color: var(--ink); font-weight: 500; }
  .history li.liked { color: var(--ink); }
  .history li.live::before { content: ""; position: absolute; left: 0; top: 4px; bottom: 4px; width: 2px; background: var(--hue); }
  .history li.liked::after {
    content: ""; position: absolute; inset-inline-start: 0; top: 4px; bottom: 4px; width: 3px;
    background: linear-gradient(90deg, var(--sodium) 0 1px, transparent 1px 2px, var(--sodium) 2px 3px);
  }
  .history li.live.liked::after { left: 5px; }
  .history li.liked :global(.heart) { color: var(--sodium); }
  .track { flex: 1; min-width: 0; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .when { flex: none; width: 48px; text-align: right; }
  .unavailable { color: var(--ink-dim); }

  .spinner {
    display: inline-block; width: 12px; height: 12px; flex: none;
    border: 2px solid currentColor; border-inline-end-color: transparent; border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) {
    .spinner { animation: none; }
  }
  @media (max-width: 720px) {
    .body { padding-inline-end: 16px; }
    .plate { padding: 16px 16px 0 20px; }
    .art { width: min(216px, 100%); height: auto; aspect-ratio: 1; }
    h1 { font-size: var(--text-xl); }
  }
</style>
