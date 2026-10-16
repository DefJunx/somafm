<script lang="ts">
  import { onMount } from "svelte";
  import type { Quality } from "../lib/api";
  import { nowPlayingText, playerStore, primaryAction, runPrimaryAction, statusText } from "../lib/player.svelte";
  import { stationsStore } from "../lib/stations.svelte";

  let { onback, player = playerStore, stations = stationsStore }: {
    onback?: () => void;
    player?: typeof playerStore;
    stations?: typeof stationsStore;
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
  });

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
      <div class="volume">
        <button onclick={toggleMute} aria-pressed={muted}>{muted ? "Unmute" : "Mute"}</button>
        <input
          type="range" min="0" max="1" step="0.01" aria-label="Volume"
          value={s.volume}
          oninput={(e) => player.setVolume(e.currentTarget.valueAsNumber)}
        />
      </div>
      <label class="quality">
        Quality
        <select value={s.quality} onchange={(e) => player.setQuality(e.currentTarget.value as Quality)}>
          <option value="mp3">MP3</option>
          <option value="aac128">AAC 128</option>
        </select>
      </label>
    </section>
  </div>
</main>

<style>
  main { padding: var(--size-4); }
  .layout { display: grid; grid-template-columns: minmax(200px, 1fr) 1.5fr; gap: var(--size-6); margin-block-start: var(--size-4); }
  .art { aspect-ratio: 1; background: var(--surface-2); border-radius: var(--radius-2); overflow: hidden; display: grid; place-items: center; }
  .art img { width: 100%; height: 100%; object-fit: cover; }
  .fallback { font-weight: var(--font-weight-6); padding: var(--size-3); text-align: center; }
  .info { display: grid; gap: var(--size-3); align-content: start; }
  .volume, .quality { display: flex; gap: var(--size-3); align-items: center; }
  .spinner {
    display: inline-block; width: 1em; height: 1em; margin-inline-end: var(--size-2);
    border: 2px solid currentColor; border-inline-end-color: transparent; border-radius: 50%;
    animation: spin 0.8s linear infinite; vertical-align: -0.15em;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
