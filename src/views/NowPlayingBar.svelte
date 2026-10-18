<script lang="ts">
  import { nowPlayingText, playerStore, primaryAction, runPrimaryAction, statusLabel, statusText } from "../lib/player.svelte";
  import { stationsStore } from "../lib/stations.svelte";
  import Icon from "../lib/Icon.svelte";
  import StatusBadge from "../lib/StatusBadge.svelte";

  let { onopenplayer, player = playerStore, stations = stationsStore }: {
    onopenplayer?: () => void;
    player?: typeof playerStore;
    stations?: typeof stationsStore;
  } = $props();

  const state = $derived(player.state);
  const station = $derived(stations.stations.find((candidate) => candidate.id === state.stationId));
  const title = $derived(station?.title ?? state.stationId ?? "");
  // The badge carries connecting and error text, so the track line only shows once playing or paused.
  const kind = $derived(state.status.kind);
  const line = $derived(kind === "connecting" || kind === "error" ? "" : nowPlayingText(state, title));
  const action = $derived(primaryAction(state));
</script>

{#if state.stationId}
  <aside class="bar" aria-label="Now playing">
    <button class="info" onclick={() => onopenplayer?.()} aria-label={`Open player for ${title}`}>
      <strong {title}>{title}</strong>
      <span class="line" title={line}>{line}</span>
    </button>
    <StatusBadge {kind} label={statusLabel(state)} text={statusText(state) || statusLabel(state)} />
    <button class:primary={action.label === "Play"} onclick={() => runPrimaryAction(player)} aria-label={action.label}>
      <Icon name={action.label === "Stop" ? "stop" : action.label === "Play" ? "play" : "warning"} />
      <span>{action.label}</span>
    </button>
  </aside>
{/if}

<style>
  .bar {
    position: fixed; inset-inline: 0; inset-block-end: 0;
    display: flex; align-items: center; gap: var(--size-3);
    padding: var(--size-2) var(--size-4);
    background: var(--surface-raised); border-block-start: 1px solid var(--surface-control);
  }
  .info { flex: 1; min-width: 0; display: grid; text-align: start; background: none; border: none; cursor: pointer; padding: 0; color: inherit; transition: color var(--motion-duration-fast) var(--motion-easing); }
  .info:hover { color: var(--color-accent); }
  .info:focus-visible { outline: var(--focus-ring); outline-offset: 2px; }
  .info strong, .line { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .info strong { font-family: var(--font-heading); color: var(--color-heading); }
  .line { font-family: var(--font-data); color: var(--text-muted); font-size: var(--font-size-1); min-height: 1lh; }
  .bar button:last-child {
    background: var(--surface-control-strong);
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: var(--size-2);
    padding: var(--size-2) var(--size-3);
  }
  .bar button:last-child:not(:disabled):hover {
    background-color: var(--color-accent);
  }
  .bar button.primary { background: var(--color-primary); color: white; }
  .bar button.primary:not(:disabled):hover { background-color: var(--color-primary); filter: brightness(1.1); }
  .bar button:last-child:focus-visible {
    outline: var(--focus-ring);
    outline-offset: 2px;
  }
</style>
