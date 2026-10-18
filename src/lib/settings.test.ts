import { afterEach, expect, it } from "vitest";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import { createSettingsStore } from "./settings.svelte";
import { favouritesIn } from "./genres";
import { station } from "./fixtures";

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
