import { afterEach, expect, it, vi } from "vitest";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import { createSettingsStore } from "./settings.svelte";
import { favouritesIn } from "./genres";
import { station } from "./fixtures";
import type { SettingsView } from "./api";

afterEach(() => clearMocks());

it("starts empty and loads get_settings on init", async () => {
  mockIPC((cmd) => (cmd === "get_settings" ? { favourites: ["a"], favouritesOnly: true } : undefined), { shouldMockEvents: true });
  const s = createSettingsStore();
  expect(s.favourites).toEqual([]);
  expect(s.favouritesOnly).toBe(false);
  await s.init();
  expect(s.favourites).toEqual(["a"]);
  expect(s.favouritesOnly).toBe(true);
});

it("replaces state wholesale on settings-changed", async () => {
  mockIPC(() => ({ favourites: [], favouritesOnly: false }), { shouldMockEvents: true });
  const s = createSettingsStore();
  await s.init();
  await emit("settings-changed", { favourites: ["b"], favouritesOnly: true });
  await new Promise((r) => setTimeout(r, 0));
  expect(s.favourites).toEqual(["b"]);
  expect(s.favouritesOnly).toBe(true);
});

it("favouritesIn keeps feed order and hides ids missing from the feed", () => {
  const feed = [station("a"), station("b"), station("c")];
  expect(favouritesIn(feed, ["c", "gone", "a"]).map((x) => x.id)).toEqual(["a", "c"]);
});

const view = (over: Partial<SettingsView> = {}): SettingsView => ({ favourites: [], favouritesOnly: false, fadeAudio: true, showVisualizer: true, ...over });

it("exposes fadeAudio and showVisualizer from get_settings and settings-changed", async () => {
  mockIPC((cmd) => (cmd === "get_settings" ? view({ fadeAudio: false }) : undefined), { shouldMockEvents: true });
  const s = createSettingsStore();
  expect(s.fadeAudio).toBe(true);
  expect(s.showVisualizer).toBe(true);
  await s.init();
  expect(s.fadeAudio).toBe(false);
  await emit("settings-changed", view({ showVisualizer: false }));
  await new Promise((r) => setTimeout(r, 0));
  expect(s.fadeAudio).toBe(true);
  expect(s.showVisualizer).toBe(false);
});

it("setFadeAudio and setShowVisualizer invoke their commands", async () => {
  const calls: [string, unknown][] = [];
  mockIPC((cmd, args) => { calls.push([cmd, args]); }, { shouldMockEvents: true });
  const s = createSettingsStore();
  await s.setFadeAudio(false);
  await s.setShowVisualizer(false);
  expect(calls).toEqual([["set_fade_audio", { value: false }], ["set_show_visualizer", { value: false }]]);
});

it.each([true, false])("init reports prefers-reduced-motion (%s) to Rust once, before reading settings", async (reduced) => {
  const calls: string[] = [];
  const args: unknown[] = [];
  vi.stubGlobal("matchMedia", (q: string) => ({ matches: q.includes("reduce") && reduced }));
  try {
    mockIPC((cmd, a) => { calls.push(cmd); if (cmd === "apply_reduced_motion_default") args.push(a); return cmd === "get_settings" ? view() : undefined; }, { shouldMockEvents: true });
    const s = createSettingsStore();
    await s.init();
    await s.init();
    expect(calls.filter((c) => c === "apply_reduced_motion_default")).toHaveLength(1);
    expect(args).toEqual([{ reduced }]);
    expect(calls.indexOf("apply_reduced_motion_default")).toBeLessThan(calls.indexOf("get_settings"));
  } finally {
    vi.unstubAllGlobals();
  }
});
