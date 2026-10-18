<script lang="ts">
  import { barLineText, playerStore, primaryAction, runPrimaryAction } from "../lib/player.svelte";
  import { stationsStore } from "../lib/stations.svelte";

  let { onopenplayer, player = playerStore, stations = stationsStore }: {
    onopenplayer?: () => void;
    player?: typeof playerStore;
    stations?: typeof stationsStore;
  } = $props();

  const state = $derived(player.state);
  const station = $derived(stations.stations.find((candidate) => candidate.id === state.stationId));
  const title = $derived(station?.title ?? state.stationId ?? "");
  const line = $derived(barLineText(state, title));
  const action = $derived(primaryAction(state));
</script>

{#if state.stationId}
  <aside class="bar" aria-label="Now playing">
    <button class="info" onclick={() => onopenplayer?.()} aria-label={`Open player for ${title}`}>
      <strong>{title}</strong>
      <span class="line">{line}</span>
    </button>
    <button onclick={() => runPrimaryAction(player)}>{action.label}</button>
  </aside>
{/if}

<style>
  .bar {
    position: fixed; inset-inline: 0; inset-block-end: 0;
    display: flex; align-items: center; gap: var(--size-3);
    padding: var(--size-2) var(--size-4);
    background: var(--surface-raised); border-block-start: 1px solid var(--surface-control);
  }
  .info { flex: 1; display: grid; text-align: start; background: none; border: none; cursor: pointer; padding: 0; color: inherit; }
  .info strong { font-family: var(--font-heading); color: var(--color-heading); }
  .line { font-family: var(--font-data); color: var(--text-muted); font-size: var(--font-size-1); }
  .bar button:last-child {
    background: var(--surface-control-strong);
    color: var(--text-primary);
    border: none;
    border-radius: var(--radius-control);
    padding: var(--size-2) var(--size-3);
    cursor: pointer;
  }
</style>
