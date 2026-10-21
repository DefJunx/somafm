<script lang="ts">
  import { onSpectrum } from "./api";

  let { active, playing }: { active: boolean; playing: boolean } = $props();

  const BANDS = 32;
  const HEIGHT = 48; // CSS px
  const GAP = 2;
  const MIN_BAR = 2; // a flat strip stays visible

  let canvas: HTMLCanvasElement | undefined = $state();
  let width = $state(0); // CSS px
  let last: number[] = [];

  function draw(bands: number[]) {
    last = bands;
    const ctx = canvas?.getContext("2d");
    if (!canvas || !ctx || width === 0) return;
    const dpr = window.devicePixelRatio || 1;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, width, HEIGHT);
    ctx.fillStyle = getComputedStyle(canvas).getPropertyValue("--color-accent").trim();
    const bar = (width - GAP * (BANDS - 1)) / BANDS;
    for (let i = 0; i < BANDS; i++) {
      const h = Math.max(MIN_BAR, Math.min(1, bands[i] ?? 0) * HEIGHT);
      ctx.fillRect(i * (bar + GAP), HEIGHT - h, bar, h);
    }
  }

  const flat = () => draw(new Array(BANDS).fill(0));

  // Responsive: the backing store follows the CSS width and the device pixel ratio.
  $effect(() => {
    if (!canvas) return;
    const el = canvas;
    const measure = () => {
      width = el.clientWidth;
      const dpr = window.devicePixelRatio || 1;
      el.width = Math.round(width * dpr);
      el.height = Math.round(HEIGHT * dpr);
      draw(last);
    };
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  });

  // No draw loop: one draw per event, only while the screen is active and playing; otherwise settle flat.
  $effect(() => {
    if (!canvas) return;
    if (!active || !playing) {
      flat();
      return;
    }
    let off = false;
    let unlisten: (() => void) | undefined;
    onSpectrum((b) => { if (!off) draw(b); }).then((u) => (off ? u() : (unlisten = u))).catch(() => {});
    return () => { off = true; unlisten?.(); };
  });
</script>

<canvas bind:this={canvas} aria-hidden="true"></canvas>

<style>
  canvas { display: block; width: 100%; height: 48px; }
</style>
