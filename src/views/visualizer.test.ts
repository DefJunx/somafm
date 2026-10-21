import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { render, cleanup, waitFor } from "@testing-library/svelte";
import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import Player from "./Player.svelte";
import { createStationsStore } from "../lib/stations.svelte";
import { createPlayerStore, initialPlayerState } from "../lib/player.svelte";
import { createSettingsStore } from "../lib/settings.svelte";
import { station } from "../lib/fixtures";
import type { PlayerState, SettingsView } from "../lib/api";

const TEAL = "#1a9096";
const playing: PlayerState = { ...initialPlayerState, stationId: "a", status: { kind: "playing" } };
type Call = [string, unknown];

const ctx = {
  setTransform: vi.fn(), clearRect: vi.fn(), fillRect: vi.fn(), fillStyle: "" as string,
};

beforeEach(() => {
  for (const f of [ctx.setTransform, ctx.clearRect, ctx.fillRect]) f.mockClear();
  ctx.fillStyle = "";
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(ctx as unknown as CanvasRenderingContext2D);
  vi.spyOn(HTMLCanvasElement.prototype, "clientWidth", "get").mockReturnValue(320);
  // jsdom does not resolve custom properties through the cascade.
  vi.spyOn(window, "getComputedStyle").mockReturnValue({ getPropertyValue: (n: string) => (n === "--color-accent" ? ` ${TEAL}` : "") } as CSSStyleDeclaration);
  vi.stubGlobal("devicePixelRatio", 2);
});
afterEach(() => {
  cleanup(); clearMocks(); vi.restoreAllMocks(); vi.unstubAllGlobals();
});

function setup(opts: { state?: PlayerState; settings?: Partial<SettingsView> } = {}) {
  const calls: Call[] = [];
  mockIPC(
    (cmd, args) => {
      if (cmd === "get_settings") return { favourites: [], favouritesOnly: false, fadeAudio: true, showVisualizer: true, ...opts.settings };
      if (cmd === "get_player_state") return opts.state ?? playing;
      if (cmd === "get_stations" || cmd === "refresh_stations") return { loaded: true, stations: [station("a")] };
      if (cmd === "get_station_history") return [];
      if (cmd === "list_liked") return [];
      calls.push([cmd, args]);
    },
    { shouldMockEvents: true },
  );
  const view = render(Player, { player: createPlayerStore(), stations: createStationsStore(), settings: createSettingsStore() });
  return { calls, view };
}

const activations = (calls: Call[]) => calls.filter(([c]) => c === "set_visualizer_active").map(([, a]) => (a as { active: boolean }).active);
const canvas = () => document.querySelector("canvas");
const bands = (f: (i: number) => number) => Array.from({ length: 32 }, (_, i) => f(i));

it("activates on mount and deactivates on unmount", async () => {
  const { calls, view } = setup();
  await waitFor(() => expect(activations(calls)).toEqual([true]));
  view.unmount();
  expect(activations(calls)).toEqual([true, false]);
});

it("deactivates when the window is hidden and reactivates when shown", async () => {
  const { calls } = setup();
  await waitFor(() => expect(activations(calls)).toEqual([true]));
  const hidden = vi.spyOn(document, "hidden", "get");
  hidden.mockReturnValue(true);
  document.dispatchEvent(new Event("visibilitychange"));
  await waitFor(() => expect(activations(calls)).toEqual([true, false]));
  hidden.mockReturnValue(false);
  document.dispatchEvent(new Event("visibilitychange"));
  await waitFor(() => expect(activations(calls)).toEqual([true, false, true]));
});

it("draws one bar per band from a spectrum event, scaled for DPR, in the accent token", async () => {
  setup();
  await waitFor(() => expect(canvas()).not.toBeNull());
  expect(canvas()!.height).toBe(96);
  expect(canvas()!.width).toBe(640);
  await waitFor(() => expect(ctx.fillRect).toHaveBeenCalled()); // initial flat frame
  ctx.fillRect.mockClear();
  // The listener attaches once the player state says playing, so keep emitting until it hears us.
  await waitFor(async () => {
    ctx.fillRect.mockClear();
    await emit("spectrum", bands((i) => (i === 3 ? 1 : 0.5)));
    expect(ctx.fillRect).toHaveBeenCalledTimes(32);
  });
  expect(ctx.fillStyle).toBe(TEAL);
  expect(ctx.setTransform).toHaveBeenCalledWith(2, 0, 0, 2, 0, 0);
  const h = ctx.fillRect.mock.calls.map((c) => c[3] as number);
  expect(h[3]).toBeCloseTo(48);
  expect(h[4]).toBeCloseTo(24);
  const xs = ctx.fillRect.mock.calls.map((c) => c[0] as number);
  expect(xs).toEqual([...xs].sort((a, b) => a - b));
  expect(xs[31]).toBeLessThan(320);
});

it("settles flat when playback stops and ignores spectrum events while not playing", async () => {
  const { view } = setup({ state: { ...playing, status: { kind: "stopped" } } });
  await waitFor(() => expect(canvas()).not.toBeNull());
  await waitFor(() => expect(ctx.fillRect.mock.calls.length).toBeGreaterThanOrEqual(32)); // flat
  const flat = ctx.fillRect.mock.calls.map((c) => c[3] as number);
  expect(new Set(flat).size).toBe(1);
  ctx.fillRect.mockClear();
  await emit("spectrum", bands(() => 1));
  await new Promise((r) => setTimeout(r, 30));
  expect(ctx.fillRect).not.toHaveBeenCalled();
  view.unmount();
});

it("is hidden and never draws when Show visualizer is off", async () => {
  const { calls } = setup({ settings: { showVisualizer: false } });
  await waitFor(() => expect(activations(calls)).toEqual([true]));
  await waitFor(() => expect(canvas()).toBeNull()); // the store starts true until settings load
  ctx.fillRect.mockClear();
  await emit("spectrum", bands(() => 1));
  await new Promise((r) => setTimeout(r, 30));
  expect(ctx.fillRect).not.toHaveBeenCalled();
});
