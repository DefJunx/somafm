<script lang="ts">
  import { nowPlayingText, playerStore, primaryAction, runPrimaryAction, statusLabel, statusText } from "../lib/player.svelte";
  import { stationsStore } from "../lib/stations.svelte";
  import { likedStore } from "../lib/liked.svelte";
  import { stationHueVar } from "../lib/genres";
  import HeartButton from "../lib/HeartButton.svelte";
  import StatusBadge from "../lib/StatusBadge.svelte";

  let { onopenplayer, player = playerStore, stations = stationsStore, liked = likedStore }: {
    onopenplayer?: () => void;
    player?: typeof playerStore;
    stations?: typeof stationsStore;
    liked?: typeof likedStore;
  } = $props();

  const snap = $derived(player.state);
  const station = $derived(stations.stations.find((candidate) => candidate.id === snap.stationId));
  const title = $derived(station?.title ?? snap.stationId ?? "");
  // The badge carries connecting and error text, so the track line only shows once playing or paused.
  const kind = $derived(snap.status.kind);
  const line = $derived(kind === "connecting" || kind === "error" ? "" : nowPlayingText(snap, title));
  const action = $derived(primaryAction(snap));
  const hue = $derived(stationHueVar(station));
  let imageFailed = $state(false);
  $effect(() => { void station?.image; imageFailed = false; });
</script>

{#if snap.stationId}
  <aside class="bar" aria-label="Now playing" style:--hue={hue}>
    <button class="info" onclick={() => onopenplayer?.()} aria-label={`Open player for ${title}`}>
      <span class="thumb" aria-hidden="true">
        {#if station?.image && !imageFailed}<img src={station.image} alt="" onerror={() => (imageFailed = true)} />{/if}
      </span>
      <strong {title}>{title}</strong>
      <span class="line" title={line}>{line}</span>
    </button>
    {#if snap.nowPlaying}
      <HeartButton store={liked} stationId={snap.stationId} raw={snap.nowPlaying.raw} />
    {/if}
    <StatusBadge {kind} label={statusLabel(snap)} text={statusText(snap) || statusLabel(snap)} />
    <button class:primary={action.label === "Play"} onclick={() => runPrimaryAction(player)} aria-label={action.label}>
      <span>{action.label}</span>
    </button>
  </aside>
{/if}

<style>
  .bar {
    position: fixed; inset-inline: 0; inset-block-end: 0; z-index: 4;
    display: flex; align-items: center; gap: 16px;
    box-sizing: border-box; max-width: 100%; height: var(--bar-h); padding: 0 24px;
    background: var(--ground); border-block-start: 1px solid var(--hairline);
  }
  .info {
    flex: 1; min-width: 0; height: 100%; display: flex; align-items: center; gap: 16px; padding: 0; border: none; border-radius: 0;
    text-align: start; color: inherit; justify-content: flex-start;
  }
  .info:not(:disabled):hover strong { text-decoration: underline; text-decoration-thickness: 1px; text-underline-offset: 4px; }
  .thumb { position: relative; flex: none; width: 32px; height: 32px; box-sizing: border-box; overflow: hidden; background: var(--well); border: 1px solid var(--hairline); }
  .thumb::after { content: ""; position: absolute; left: 0; right: 0; bottom: 0; height: 2px; background: var(--hue); }
  .thumb img { display: block; width: 100%; height: 100%; object-fit: cover; }
  .info strong { flex: none; max-width: 40%; font-size: var(--text-md); line-height: 24px; font-weight: 700; color: var(--ink); overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .line { flex: 1; min-width: 0; font-size: var(--text-md); line-height: 24px; color: var(--ink-dim); overflow: hidden; white-space: nowrap; text-overflow: ellipsis; letter-spacing: 0; }
  .bar > :global(*) { min-width: 0; }
  .bar > :global(.badge) { flex: 0 1 auto; }
  .bar > button:last-child { flex: none; min-width: 80px; }
  @media (max-width: 720px) {
    .bar { padding: 0 16px; gap: 8px; }
    .info { gap: 8px; }
  }
  @media (max-width: 600px) {
    .line { display: none; }
    .info strong { max-width: none; flex: 1; }
  }
</style>
