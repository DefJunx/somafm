<script lang="ts">
  import { onMount } from "svelte";
  import { openUrl } from "../lib/api";
  import { likedStore } from "../lib/liked.svelte";
  import { playerStore } from "../lib/player.svelte";
  import { stationsStore } from "../lib/stations.svelte";
  import { rawText, searchLinks } from "../lib/tracks";
  import Icon from "../lib/Icon.svelte";
  import NowPlayingBar from "./NowPlayingBar.svelte";

  let { onback, onopenplayer, player = playerStore, stations = stationsStore, liked = likedStore }: {
    onback?: () => void;
    onopenplayer?: () => void;
    player?: typeof playerStore;
    stations?: typeof stationsStore;
    liked?: typeof likedStore;
  } = $props();

  onMount(() => {
    player.init().catch(() => {});
    stations.init().catch(() => {});
    liked.init().catch(() => {});
  });

  const dateFormat = new Intl.DateTimeFormat(undefined, { dateStyle: "medium" });
  const stationName = (id: string) => stations.stations.find((s) => s.id === id)?.title;
  const copy = (raw: string) => navigator.clipboard?.writeText(raw).catch(() => {});
</script>

<main>
  <button class="back" aria-label="Back" onclick={() => onback?.()}>
    <Icon name="back" />
    <span>Back</span>
  </button>
  <h1 tabindex="-1">Liked</h1>
  {#if liked.tracks.length === 0}
    <p class="empty">No liked tracks yet</p>
  {:else}
    <ul>
      {#each liked.tracks as t (`${t.stationId}\n${t.raw}`)}
        {@const name = stationName(t.stationId)}
        <li>
          <div class="main">
            <span class="track" title={rawText(t.raw)}>{rawText(t.raw)}</span>
            <span class="meta">
              {#if name}
                <button class="station" onclick={() => player.play(t.stationId).catch(() => {})}>{name}</button>
              {:else}
                <span class="station missing">{t.stationId}</span>
              {/if}
              <time datetime={new Date(t.likedAt).toISOString()}>{dateFormat.format(t.likedAt)}</time>
            </span>
          </div>
          <div class="actions">
            <button onclick={() => copy(t.raw)}>Copy text</button>
            <span class="search" role="group" aria-label="Search on">
              <span class="label" aria-hidden="true">Search on</span>
              {#each searchLinks(t.raw) as link (link.name)}
                <button onclick={() => openUrl(link.url).catch(() => {})}>{link.name}</button>
              {/each}
            </span>
            <button onclick={() => liked.remove(t.stationId, t.raw).catch(() => {})}>Remove</button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</main>
<NowPlayingBar {onopenplayer} {player} {stations} {liked} />

<style>
  main { padding: var(--size-4); padding-block-end: var(--size-10); min-width: 0; }
  h1 { font-family: var(--font-heading); color: var(--color-heading); margin-block: var(--size-4); }
  h1:focus { outline: none; }
  .back { display: flex; align-items: center; gap: var(--size-2); }
  .empty { font-family: var(--font-data); color: var(--text-muted); }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: var(--size-2); }
  li { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: var(--size-2) var(--size-4); padding: var(--size-3); background: var(--surface-raised); border-radius: var(--radius-2); }
  .main { display: grid; min-width: 0; flex: 1 1 16rem; }
  .track { font-family: var(--font-data); color: var(--text-primary); overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .meta { display: flex; gap: var(--size-3); font-size: var(--font-size-0); color: var(--text-muted); }
  .station { background: none; border: none; padding: 0; color: var(--text-muted); cursor: pointer; text-decoration: underline; font: inherit; }
  .station:not(.missing):hover { color: var(--color-accent); }
  .station.missing { text-decoration: none; }
  .station:focus-visible { outline: var(--focus-ring); outline-offset: 2px; }
  .actions, .search { display: flex; flex-wrap: wrap; align-items: center; gap: var(--size-2); }
  .label { color: var(--text-muted); font-size: var(--font-size-0); }
  .back, .actions button { background: var(--surface-control-strong); color: var(--text-primary); }
  .back:not(:disabled):hover, .actions button:not(:disabled):hover { background-color: var(--color-accent); }
</style>
