<script lang="ts">
  import type { Station } from "./api";
  import { stationHueVar } from "./genres";

  type Kind = "playing" | "stopped" | "connecting" | "error";
  /** The tuning rail: one tick per Station in feed order; the loaded Station is bold with its tick reaching into the plate. */
  let { stations, activeId, kind, reconnecting = false, onselect }: {
    stations: Station[];
    activeId: string | null;
    kind: Kind;
    reconnecting?: boolean;
    onselect: (station: Station) => void;
  } = $props();

  const word = $derived(
    kind === "playing" ? "live"
      : kind === "connecting" ? (reconnecting ? "reconnecting" : "connecting")
      : kind === "error" ? "error"
      : "paused",
  );
</script>

<nav class="rail" aria-label="Stations">
  <ul>
    {#each stations as s (s.id)}
      {@const current = s.id === activeId}
      <li>
        <button
          class="entry"
          class:current
          class:dashed={current && reconnecting}
          class:short={current && kind === "error"}
          style:--hue={stationHueVar(s)}
          title={s.title}
          aria-label={`Play ${s.title}`}
          aria-current={current ? "true" : undefined}
          onclick={() => onselect(s)}
        >
          <span class="name" title={s.title}>{s.title}</span>
          {#if current}<em class="state">{word}</em>{/if}
        </button>
      </li>
    {/each}
  </ul>
</nav>

<style>
  .rail::after { content: ""; position: absolute; right: 0; top: 0; bottom: 0; width: 1px; background: var(--hairline); }
  .rail { position: relative; z-index: 1; min-width: 0; padding-block: 16px; }
  ul { list-style: none; margin: 0; padding: 0; }
  .entry {
    --hue-ring: var(--hue);
    position: relative; display: flex; align-items: center; justify-content: flex-end; gap: 8px;
    width: 100%; height: 16px; min-height: 16px; padding: 0 32px 0 0; border: none; border-radius: 0;
    color: var(--ink-dim); font-weight: 400; letter-spacing: 0.02em; text-align: right;
  }
  .entry:not(:disabled):hover { color: var(--ink); }
  .entry::after {
    content: ""; position: absolute; right: 0; top: 7px; width: 20px; height: 1px; background: var(--hue);
    transition: width var(--motion-duration-base) var(--motion-easing);
  }
  .name { min-width: 0; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .current { color: var(--ink); font-weight: 700; }
  .current::after { right: calc(-1 * var(--rail-gap)); width: calc(20px + var(--rail-gap)); height: 2px; top: 6px; filter: drop-shadow(0 0 4px color-mix(in srgb, var(--hue) 35%, transparent)); }
  .current.short::after { width: 6px; right: 0; }
  .current.dashed::after { background: repeating-linear-gradient(90deg, var(--hue) 0 4px, transparent 4px 7px); }
  .state { flex: none; font-style: normal; font-weight: 500; letter-spacing: var(--track-word); text-transform: uppercase; color: var(--ink-dim); }
  .current:not(.dashed):not(.short) .state { color: var(--ink); }

  /* Narrow: ticks only. The names stay in the accessible name. */
  @media (max-width: 720px) {
    .entry { padding: 0; }
    .name, .state { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); }
    .entry:hover .name, .entry:focus-visible .name {
      left: calc(100% + 8px); top: -4px; width: auto; height: 24px; line-height: 24px; padding: 0 8px; clip-path: none;
      background: var(--plate); border: 1px solid var(--hairline); border-radius: var(--radius); color: var(--ink); z-index: 2;
    }
  }
</style>
