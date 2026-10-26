<script lang="ts">
  import { onSpectrum } from "./api";

  /** `hue` is a CSS colour value (a Station hue token); it lands on the canvas as `--hue`, which is what gets drawn. */
  let { active, playing, hue = "var(--ink-dim)" }: { active: boolean; playing: boolean; hue?: string } = $props();

  const BANDS = 32;
  const HEIGHT = 56; // CSS px
  const MIN_LINE = 2; // a flat strip stays visible

  let canvas: HTMLCanvasElement | undefined = $state();
  let width = $state(0); // CSS px
  let last: number[] = [];
  let color = "";

  function draw(bands: number[]) {
    last = bands;
    const ctx = canvas?.getContext("2d");
    if (!canvas || !ctx || width === 0) return;
    const dpr = window.devicePixelRatio || 1;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, width, HEIGHT);
    ctx.fillStyle = color;
    // One 1px emission line per band, spread edge to edge.
    const step = (width - 1) / (BANDS - 1);
    for (let i = 0; i < BANDS; i++) {
      const h = Math.max(MIN_LINE, Math.min(1, bands[i] ?? 0) * HEIGHT);
      ctx.fillRect(Math.round(i * step), HEIGHT - h, 1, h);
    }
  }

  const flat = () => draw(new Array(BANDS).fill(0));

  // Responsive: the backing store follows the CSS width and the device pixel ratio.
  $effect(() => {
    if (!canvas) return;
    const el = canvas;
    const measure = () => {
      width = el.clientWidth;
      color = getComputedStyle(el).getPropertyValue("--hue").trim(); // once per resize, not per draw
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

  // A new Station hue re-reads the colour; an unchanged colour costs no draw.
  $effect(() => {
    void hue;
    if (!canvas) return;
    const next = getComputedStyle(canvas).getPropertyValue("--hue").trim();
    if (next === color) return;
    color = next;
    draw(last);
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

<canvas bind:this={canvas} style:height="{HEIGHT}px" style:--hue={hue} aria-hidden="true"></canvas>

<style>
  canvas { display: block; width: 100%; }
</style>
