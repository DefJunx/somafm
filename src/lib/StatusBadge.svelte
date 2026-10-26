<script lang="ts">
  import Icon from "./Icon.svelte";

  /** `label` names the status for assistive tech; `text` is the visible text (defaults to `label`). */
  let { kind, label, text = label, ...rest }: {
    kind: "playing" | "stopped" | "connecting" | "error";
    label: string;
    text?: string;
    [key: string]: unknown;
  } = $props();

  const reconnecting = $derived(kind === "connecting" && label === "Reconnecting");
</script>

<span class="badge" role="group" data-status={kind} aria-label={`Status: ${label}`} {...rest}>
  {#if kind === "error"}<Icon name="warning" />{:else}<i class="dot" class:still={kind === "stopped"} class:dashed={reconnecting}></i>{/if}<span class="text">{text}</span>
</span>

<style>
  .badge {
    display: inline-flex; align-items: center; gap: 8px; flex: none; max-width: 100%;
    color: var(--ink); font-size: var(--text-sm); line-height: 16px; font-weight: 500;
    letter-spacing: var(--track-word); text-transform: uppercase;
  }
  .badge[data-status="stopped"] { color: var(--ink-dim); }
  .text { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; min-width: 0; }
  .dot { flex: none; display: block; width: 16px; height: 2px; background: var(--hue); animation: pulse 1.4s ease-in-out infinite; }
  .dot.still { animation: none; background: var(--ink-dim); }
  .dot.dashed { background: repeating-linear-gradient(90deg, var(--hue) 0 4px, transparent 4px 7px); }
  @keyframes pulse { 50% { opacity: 0.35; } }
  @media (prefers-reduced-motion: reduce) {
    .dot { animation: none; }
  }
</style>
