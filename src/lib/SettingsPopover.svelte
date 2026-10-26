<script lang="ts">
  import { tick } from "svelte";
  import { settingsStore } from "./settings.svelte";
  import Icon from "./Icon.svelte";

  let { settings = settingsStore }: { settings?: typeof settingsStore } = $props();

  let open = $state(false);
  let root: HTMLElement | undefined = $state();
  let gear: HTMLButtonElement | undefined = $state();
  let firstToggle: HTMLInputElement | undefined = $state();

  async function toggle() {
    open = !open;
    if (open) {
      await tick();
      firstToggle?.focus();
    }
  }

  function close(returnFocus: boolean) {
    open = false;
    if (returnFocus) gear?.focus();
  }

  function onkeydown(e: KeyboardEvent) {
    if (open && e.key === "Escape") {
      e.stopPropagation();
      close(true);
    }
  }

  function onpointerdown(e: PointerEvent) {
    if (open && root && !root.contains(e.target as Node)) close(false);
  }
</script>

<svelte:window {onpointerdown} />

<div class="settings" bind:this={root} {onkeydown} role="presentation">
  <button class="gear" bind:this={gear} aria-label="Settings" aria-haspopup="dialog" aria-expanded={open} onclick={toggle}>
    <Icon name="settings" />
  </button>
  {#if open}
    <div class="popover" role="dialog" aria-label="Settings">
      <label>
        <input type="checkbox" bind:this={firstToggle} checked={settings.fadeAudio} onchange={(e) => settings.setFadeAudio(e.currentTarget.checked).catch(() => {})} />
        <span>Fade audio</span>
      </label>
      <label>
        <input type="checkbox" checked={settings.showVisualizer} onchange={(e) => settings.setShowVisualizer(e.currentTarget.checked).catch(() => {})} />
        <span>Show visualizer</span>
      </label>
    </div>
  {/if}
</div>

<style>
  .settings { position: relative; }
  .gear { width: var(--control-h); padding: 0; }
  .popover {
    position: absolute; inset-inline-end: 0; inset-block-start: 100%; margin-block-start: 8px; z-index: 10;
    display: grid; gap: 16px; padding: 16px 24px; white-space: nowrap;
    background: var(--plate); border: 1px solid var(--hairline); border-radius: var(--radius);
  }
  label { display: flex; align-items: center; gap: 16px; color: var(--ink); cursor: pointer; font-size: var(--text-md); line-height: 24px; }
</style>
