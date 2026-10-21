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
  .gear { display: flex; align-items: center; background: var(--surface-control-strong); color: var(--text-primary); }
  .gear:not(:disabled):hover { background-color: var(--color-accent); }
  .popover {
    position: absolute; inset-inline-end: 0; inset-block-start: 100%; margin-block-start: var(--size-2); z-index: 10;
    display: grid; gap: var(--size-3); padding: var(--size-3) var(--size-4); white-space: nowrap;
    background: var(--surface-raised); border-radius: var(--radius-control); box-shadow: 0 4px 16px rgb(0 0 0 / 0.5);
  }
  label { display: flex; align-items: center; gap: var(--size-3); color: var(--text-primary); cursor: pointer; }
  input { accent-color: var(--color-accent); cursor: pointer; }
  input:focus-visible { outline: var(--focus-ring); outline-offset: 2px; }
</style>
