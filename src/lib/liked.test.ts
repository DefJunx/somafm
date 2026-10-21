import { afterEach, expect, it } from "vitest";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import { createLikedStore } from "./liked.svelte";
import type { LikedTrack } from "./api";

afterEach(() => clearMocks());

const t = (stationId: string, raw: string, likedAt = 1): LikedTrack => ({ stationId, raw, likedAt });

it("starts empty and loads list_liked on init", async () => {
  mockIPC((cmd) => (cmd === "list_liked" ? [t("a", "x - y")] : undefined), { shouldMockEvents: true });
  const s = createLikedStore();
  expect(s.tracks).toEqual([]);
  await s.init();
  expect(s.tracks).toEqual([t("a", "x - y")]);
  expect(s.isLiked("a", "x - y")).toBe(true);
  expect(s.isLiked("b", "x - y")).toBe(false);
  expect(s.isLiked("a", "other")).toBe(false);
});

it("replaces state wholesale on liked-changed", async () => {
  mockIPC(() => [], { shouldMockEvents: true });
  const s = createLikedStore();
  await s.init();
  await emit("liked-changed", [t("a", "q - r")]);
  await new Promise((r) => setTimeout(r, 0));
  expect(s.isLiked("a", "q - r")).toBe(true);
  await emit("liked-changed", []);
  await new Promise((r) => setTimeout(r, 0));
  expect(s.isLiked("a", "q - r")).toBe(false);
});

it("toggle and remove invoke the backend commands", async () => {
  const calls: [string, unknown][] = [];
  mockIPC((cmd, args) => { calls.push([cmd, args]); return cmd === "list_liked" ? [] : cmd === "toggle_like" ? true : undefined; }, { shouldMockEvents: true });
  const s = createLikedStore();
  expect(await s.toggle("a", "x - y")).toBe(true);
  await s.remove("a", "x - y");
  expect(calls).toContainEqual(["toggle_like", { stationId: "a", raw: "x - y" }]);
  expect(calls).toContainEqual(["remove_liked", { stationId: "a", raw: "x - y" }]);
});

it("restore sends the whole track, keeping its likedAt", async () => {
  const calls: [string, unknown][] = [];
  mockIPC((cmd, args) => { calls.push([cmd, args]); }, { shouldMockEvents: true });
  await createLikedStore().restore(t("a", "x - y", 42));
  expect(calls).toContainEqual(["restore_liked", { track: { stationId: "a", raw: "x - y", likedAt: 42 } }]);
});
