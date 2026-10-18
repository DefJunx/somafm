<script lang="ts">
  import Icon from "./Icon.svelte";

  /** `label` names the status for assistive tech; `text` is the visible text (defaults to `label`). */
  let { kind, label, text = label, ...rest }: {
    kind: "playing" | "stopped" | "connecting" | "error";
    label: string;
    text?: string;
    [key: string]: unknown;
  } = $props();
</script>

<span class="badge" role="group" data-status={kind} aria-label={`Status: ${label}`} {...rest}>
  {#if kind === "error"}<Icon name="warning" />{:else}<i class="dot" class:still={kind === "stopped"}></i>{/if}<span class="text">{text}</span>
</span>

<style>
  .badge {
    display: inline-flex; align-items: center; gap: var(--size-1); flex: none; max-width: 100%;
    padding: var(--size-1) var(--size-2); border-radius: var(--radius-round);
    background: var(--surface-control); color: var(--text-primary);
    font-family: var(--font-data); font-size: var(--font-size-0); line-height: 1;
  }
  .badge[data-status="error"] { color: var(--color-heading); }
  .text { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; min-width: 0; }
  .dot { flex: none; display: inline-block; width: 0.6em; height: 0.6em; border-radius: 50%; background: currentColor; animation: pulse 1s ease-in-out infinite; }
  .dot.still { animation: none; }
  @keyframes pulse { 50% { opacity: 0.25; } }
  @media (prefers-reduced-motion: reduce) {
    .dot { animation: none; }
  }
</style>
