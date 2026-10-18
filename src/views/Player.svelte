<script lang="ts">
  import { onMount } from "svelte";
  import { nowPlayingText, playerStore, primaryAction, runPrimaryAction, statusLabel, statusText } from "../lib/player.svelte";
  import { settingsStore } from "../lib/settings.svelte";
  import { stationsStore } from "../lib/stations.svelte";
  import Icon from "../lib/Icon.svelte";
  import StatusBadge from "../lib/StatusBadge.svelte";

  let { onback, player = playerStore, stations = stationsStore, settings = settingsStore }: {
    onback?: () => void;
    player?: typeof playerStore;
    stations?: typeof stationsStore;
    settings?: typeof settingsStore;
  } = $props();

  let lastVolume = 0.8;
  let imageFailed = $state(false);

  const s = $derived(player.state);
  const station = $derived(stations.stations.find((x) => x.id === s.stationId));
  const title = $derived(station?.title ?? s.stationId ?? "");
  const muted = $derived(s.volume === 0);
  const action = $derived(primaryAction(s));

  onMount(() => {
    player.init().catch(() => {});
    stations.init().catch(() => {});
    settings.init().catch(() => {});
  });

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

<main>
  <button class="back" aria-label="Back" onclick={() => onback?.()}>
    <Icon name="back" />
    <span>Back</span>
  </button>
  <div class="layout">
    <div class="art">
      {#if station && !imageFailed}
        <img src={station.xlImage || station.largeImage} alt={station.title} onerror={() => (imageFailed = true)} />
      {:else}
        <span class="fallback">{title}</span>
      {/if}
    </div>
    <section class="info">
      <h1 {title}>{title}</h1>
      {#if station}
        <p class="desc">{station.description}</p>
        <p class="listeners">{station.listeners} listeners</p>
      {/if}
      <p class="now" title={nowPlayingText(s, title)}>{nowPlayingText(s, title)}</p>
      <p class="status" role="status"><StatusBadge kind={s.status.kind} label={statusLabel(s)} text={statusText(s) || statusLabel(s)} /></p>
      <button class="main" class:primary={action.label === "Play"} onclick={() => runPrimaryAction(player)} disabled={action.busy} aria-busy={action.busy}>
        {#if action.busy}
          <span class="spinner" aria-hidden="true"></span>
        {:else}
          <Icon name={action.label === "Stop" ? "stop" : action.label === "Play" ? "play" : "warning"} />
        {/if}
        <span>{action.busy ? "Connecting" : action.label}</span>
      </button>
      {#if s.stationId}
        <button class="fav" aria-pressed={favourite} aria-label={favourite ? "Remove from Favourites" : "Add to Favourites"} onclick={() => s.stationId && settings.toggleFavourite(s.stationId).catch(() => {})}>
          <Icon name={favourite ? "star-filled" : "star"} />
          <span>Favourite</span>
        </button>
      {/if}
      <div class="volume">
        <button onclick={toggleMute} aria-pressed={muted} aria-label={muted ? "Unmute" : "Mute"}>
          <Icon name={muted ? "mute" : "volume"} />
        </button>
        <input
          type="range" min="0" max="1" step="0.01" aria-label="Volume"
          value={s.volume}
          style:--fill="{s.volume * 100}%"
          oninput={(e) => player.setVolume(e.currentTarget.valueAsNumber)}
        />
      </div>
    </section>
  </div>
</main>

<style>
  main { padding: var(--size-4); }
  .back {
    display: flex;
    align-items: center;
    gap: var(--size-2);
  }
  .layout { display: grid; grid-template-columns: minmax(200px, 1fr) 1.5fr; gap: var(--size-6); margin-block-start: var(--size-4); }
  .art { aspect-ratio: 1; background: var(--surface-raised); border-radius: var(--radius-2); overflow: hidden; display: grid; place-items: center; }
  .art img { width: 100%; height: 100%; object-fit: cover; }
  .fallback { font-weight: var(--font-weight-6); padding: var(--size-3); text-align: center; font-family: var(--font-heading); }
  .info { display: grid; gap: var(--size-3); align-content: start; }
  .info h1 {
    font-family: var(--font-heading); color: var(--color-heading); margin: 0;
    display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 2; line-clamp: 2;
    overflow: hidden; overflow-wrap: anywhere;
  }
  .desc { color: var(--text-primary); }
  .now { font-family: var(--font-data); color: var(--text-primary); margin: 0; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .listeners { font-family: var(--font-data); color: var(--text-muted); }
  .status { margin: 0; min-width: 0; }
  .back, .main, .fav, .volume button {
    background: var(--surface-control-strong);
    color: var(--text-primary);
  }
  .back:not(:disabled):hover, .main:not(:disabled):hover, .fav:not(:disabled):hover, .volume button:not(:disabled):hover {
    background-color: var(--color-accent);
  }
  .main {
    padding: var(--size-3) var(--size-4);
    font-weight: var(--font-weight-6);
    display: flex;
    align-items: center;
    gap: var(--size-2);
  }
  .main.primary { background: var(--color-primary); color: white; }
  .main.primary:not(:disabled):hover { background-color: var(--color-primary); filter: brightness(1.1); }
  .fav {
    display: flex;
    align-items: center;
    gap: var(--size-2);
  }
  .volume { display: flex; gap: var(--size-3); align-items: center; }
  .volume button {
    padding: var(--size-2);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .spinner {
    display: inline-block; width: 1em; height: 1em; margin-inline-end: var(--size-2);
    border: 2px solid currentColor; border-inline-end-color: transparent; border-radius: 50%;
    animation: spin 0.8s linear infinite; vertical-align: -0.15em;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
