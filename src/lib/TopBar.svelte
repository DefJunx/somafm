<script lang="ts">
  import Icon from "./Icon.svelte";
  import SettingsPopover from "./SettingsPopover.svelte";
  import { settingsStore } from "./settings.svelte";

  /** The 48px bar above every screen. `current` marks where the listener is; Stations is the Grid. */
  let { current, onstations, onliked, backLabel = false, settings = settingsStore }: {
    current: "stations" | "player" | "liked";
    onstations?: () => void;
    onliked?: () => void;
    /** On the Player screen and Liked view the way to the Grid is a Back button. */
    backLabel?: boolean;
    settings?: typeof settingsStore;
  } = $props();
</script>

<header class="top">
  <span class="brand">SomaFM Player</span>
  <nav aria-label="Screens">
    {#if backLabel}
      <button class="back" aria-label="Back" onclick={() => onstations?.()}>
        <Icon name="back" />
        <span>Back</span>
      </button>
    {:else}
      <span class="here" aria-current="page">Stations</span>
    {/if}
    {#if current === "liked"}
      <span class="here" aria-current="page">Liked</span>
    {:else}
      <button class="liked" onclick={() => onliked?.()}>
        <Icon name="heart" />
        <span>Liked</span>
      </button>
    {/if}
  </nav>
  <span class="grow"></span>
  <SettingsPopover {settings} />
</header>

<style>
  .top {
    box-sizing: border-box; height: var(--topbar-h); display: flex; align-items: center; gap: 8px;
    padding: 0 24px; border-block-end: 1px solid var(--hairline);
  }
  .brand {
    flex: none; width: calc(var(--rail-w) - 24px - 8px); color: var(--ink-dim); font-weight: 500;
    letter-spacing: var(--track-label); text-transform: uppercase; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  nav { display: flex; align-items: center; gap: 8px; min-width: 0; }
  .here {
    display: inline-flex; align-items: center; height: var(--control-h); box-sizing: border-box; padding: 0 16px;
    border: 1px solid var(--ink); border-radius: var(--radius); letter-spacing: var(--track-body);
  }
  .grow { flex: 1; }
  @media (max-width: 720px) {
    .top { padding: 0 16px; }
    .brand { display: none; }
  }
  @media (max-width: 560px) {
    .back span { display: none; }
    .back { padding: 0; width: var(--control-h); }
  }
</style>
