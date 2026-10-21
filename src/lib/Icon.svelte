<script lang="ts">
  let { name }: { name: string } = $props();
</script>

{#if name === "play"}
  <svg viewBox="0 0 24 24" width="1em" height="1em" fill="currentColor" aria-hidden="true">
    <polygon points="5 3 19 12 5 21" />
  </svg>
{:else if name === "pause"}
  <svg viewBox="0 0 24 24" width="1em" height="1em" fill="currentColor" aria-hidden="true">
    <rect x="6" y="4" width="4" height="16" />
    <rect x="14" y="4" width="4" height="16" />
  </svg>
{:else if name === "stop"}
  <svg viewBox="0 0 24 24" width="1em" height="1em" fill="currentColor" aria-hidden="true">
    <rect x="5" y="5" width="14" height="14" />
  </svg>
{:else if name === "star"}
  <svg viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
    <polygon points="12 2 14.9 8.6 22 9.3 16.6 14 18.2 21 12 17.3 5.8 21 7.4 14 2 9.3 9.1 8.6" />
  </svg>
{:else if name === "star-filled"}
  <svg viewBox="0 0 24 24" width="1em" height="1em" fill="currentColor" aria-hidden="true">
    <polygon points="12 2 14.9 8.6 22 9.3 16.6 14 18.2 21 12 17.3 5.8 21 7.4 14 2 9.3 9.1 8.6" />
  </svg>
{:else if name === "heart"}
  <svg viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
    <path d="M12 21s-8-5.2-8-11a4.6 4.6 0 0 1 8-3 4.6 4.6 0 0 1 8 3c0 5.8-8 11-8 11Z" />
  </svg>
{:else if name === "heart-filled"}
  <svg viewBox="0 0 24 24" width="1em" height="1em" fill="currentColor" aria-hidden="true">
    <path d="M12 21s-8-5.2-8-11a4.6 4.6 0 0 1 8-3 4.6 4.6 0 0 1 8 3c0 5.8-8 11-8 11Z" />
  </svg>
{:else if name === "back"}
  <svg viewBox="0 0 24 24" width="1em" height="1em" fill="currentColor" aria-hidden="true">
    <path d="M15 18l-6-6 6-6" stroke="currentColor" stroke-width="2" fill="none" />
  </svg>
{:else if name === "volume"}
  <svg viewBox="0 0 24 24" width="1em" height="1em" fill="currentColor" aria-hidden="true">
    <polygon points="3 9 3 15 7 15 12 20 12 4 7 9" />
    <path d="M15 7a6 6 0 0 1 0 10" stroke="currentColor" stroke-width="2" fill="none" />
  </svg>
{:else if name === "mute"}
  <svg viewBox="0 0 24 24" width="1em" height="1em" fill="currentColor" aria-hidden="true">
    <polygon points="3 9 3 15 7 15 12 20 12 4 7 9" />
    <line x1="23" y1="1" x2="1" y2="23" stroke="currentColor" stroke-width="2" />
  </svg>
{:else if name === "copy"}
  <svg viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
    <rect x="9" y="9" width="11" height="11" rx="2" />
    <path d="M5 15V6a2 2 0 0 1 2-2h8" />
  </svg>
{:else if name === "check"}
  <svg viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
    <path d="M5 12.5l4.5 4.5L19 7.5" />
  </svg>
{:else if name === "close"}
  <svg viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
    <path d="M6 6l12 12M18 6L6 18" />
  </svg>
{:else if name === "settings"}
  <svg viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
    <circle cx="12" cy="12" r="3" />
    <path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M4.9 19.1L7 17M17 7l2.1-2.1" />
  </svg>
{:else if name === "warning"}
  <svg viewBox="0 0 24 24" width="1em" height="1em" fill="currentColor" aria-hidden="true">
    <path d="M12 2L2 20h20Z" />
    <circle cx="12" cy="16" r="1" fill="var(--surface-base)" />
    <line x1="12" y1="8" x2="12" y2="14" stroke="var(--surface-base)" stroke-width="1.5" />
  </svg>
{/if}

<style>
  svg {
    display: inline-block;
    vertical-align: -0.125em;
  }
</style>
