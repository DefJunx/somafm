<script lang="ts">
  import { onMount } from "svelte";
  import { nowPlayingText, playerStore, primaryAction, runPrimaryAction, statusText } from "../lib/player.svelte";
  import { settingsStore } from "../lib/settings.svelte";
  import { stationsStore } from "../lib/stations.svelte";

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
  <button class="back" onclick={() => onback?.()}>Back</button>
  <div class="layout">
    <div class="art">
      {#if station && !imageFailed}
        <img src={station.xlImage || station.largeImage} alt={station.title} onerror={() => (imageFailed = true)} />
      {:else}
        <span class="fallback">{title}</span>
      {/if}
    </div>
    <section class="info">
      <h1>{title}</h1>
      {#if station}
        <p class="desc">{station.description}</p>
        <p class="listeners">{station.listeners} listeners</p>
      {/if}
      <p class="now">{nowPlayingText(s, title)}</p>
      <p class="status" role="status">{statusText(s)}</p>
      <button class="main" onclick={() => runPrimaryAction(player)} disabled={action.busy} aria-busy={action.busy}>
        {#if action.busy}<span class="spinner" aria-hidden="true"></span>{/if}{action.busy ? "Connecting" : action.label}
      </button>
      {#if s.stationId}
        <button class="fav" aria-pressed={favourite} aria-label={favourite ? "Remove from Favourites" : "Add to Favourites"} onclick={() => s.stationId && settings.toggleFavourite(s.stationId).catch(() => {})}>
          {favourite ? "\u2605" : "\u2606"} Favourite
        </button>
      {/if}
      <div class="volume">
        <button onclick={toggleMute} aria-pressed={muted}>{muted ? "Unmute" : "Mute"}</button>
        <input
          type="range" min="0" max="1" step="0.01" aria-label="Volume"
          value={s.volume}
          oninput={(e) => player.setVolume(e.currentTarget.valueAsNumber)}
        />
      </div>
    </section>
  </div>
</main>

<style>
  main { padding: var(--size-4); }
  .back {
    background: var(--surface-control);
    color: var(--text-primary);
    border: none;
    border-radius: var(--radius-control);
    padding: var(--size-2) var(--size-3);
    cursor: pointer;
  }
  .layout { display: grid; grid-template-columns: minmax(200px, 1fr) 1.5fr; gap: var(--size-6); margin-block-start: var(--size-4); }
  .art { aspect-ratio: 1; background: var(--surface-raised); border-radius: var(--radius-2); overflow: hidden; display: grid; place-items: center; }
  .art img { width: 100%; height: 100%; object-fit: cover; }
  .fallback { font-weight: var(--font-weight-6); padding: var(--size-3); text-align: center; font-family: var(--font-heading); }
  .info { display: grid; gap: var(--size-3); align-content: start; }
  .info h1 { font-family: var(--font-heading); color: var(--color-heading); margin: 0; }
  .desc { color: var(--text-primary); }
  .now { font-family: var(--font-data); color: var(--text-primary); }
  .listeners { font-family: var(--font-data); color: var(--text-muted); }
  .status { font-family: var(--font-data); color: var(--text-muted); }
  .main {
    background: var(--color-primary);
    color: white;
    border: none;
    border-radius: var(--radius-control);
    padding: var(--size-3) var(--size-4);
    cursor: pointer;
    font-weight: var(--font-weight-6);
  }
  .main:disabled {
    opacity: 0.7;
    cursor: not-allowed;
  }
  .fav {
    background: var(--surface-control);
    color: var(--text-primary);
    border: none;
    border-radius: var(--radius-control);
    padding: var(--size-2) var(--size-3);
    cursor: pointer;
  }
  .volume { display: flex; gap: var(--size-3); align-items: center; }
  .volume button {
    background: var(--surface-control);
    color: var(--text-primary);
    border: none;
    border-radius: var(--radius-control);
    padding: var(--size-2) var(--size-3);
    cursor: pointer;
  }
  .spinner {
    display: inline-block; width: 1em; height: 1em; margin-inline-end: var(--size-2);
    border: 2px solid currentColor; border-inline-end-color: transparent; border-radius: 50%;
    animation: spin 0.8s linear infinite; vertical-align: -0.15em;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
