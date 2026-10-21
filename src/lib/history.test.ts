import { afterEach, expect, it } from "vitest";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { createHistoryStore } from "./history.svelte";
import type { HistoryEntry } from "./api";

afterEach(() => clearMocks());

const e = (artist: string, title: string): HistoryEntry => ({ time: 1_700_000_000, artist, title, raw: `${artist} - ${title}` });

it("loads the history for a station", async () => {
  mockIPC((cmd, args) => (cmd === "get_station_history" ? [e((args as { stationId: string }).stationId, "t")] : undefined));
  const h = createHistoryStore();
  await h.load("a");
  expect(h.entries).toEqual([e("a", "t")]);
  expect(h.failed).toBe(false);
});

it("flags a failure without retrying, and a later load can recover", async () => {
  let calls = 0;
  mockIPC(() => { calls++; if (calls === 1) throw new Error("boom"); return [e("x", "y")]; });
  const h = createHistoryStore();
  await h.load("a");
  expect(h.failed).toBe(true);
  expect(h.entries).toEqual([]);
  expect(calls).toBe(1);
  await h.load("a");
  expect(h.failed).toBe(false);
  expect(h.entries).toHaveLength(1);
});

it("discards the previous list on a station switch and ignores a stale response", async () => {
  const pending: Record<string, (v: HistoryEntry[]) => void> = {};
  mockIPC((cmd, args) => new Promise((r) => (pending[(args as { stationId: string }).stationId] = r)));
  const h = createHistoryStore();
  const first = h.load("a");
  const second = h.load("b");
  pending.b([e("b", "bt")]);
  await second;
  pending.a([e("a", "at")]);
  await first;
  expect(h.entries).toEqual([e("b", "bt")]);
});

it("clears entries immediately when the station changes", async () => {
  let release: (v: HistoryEntry[]) => void = () => {};
  let n = 0;
  mockIPC(() => (n++ === 0 ? [e("a", "t")] : new Promise((r) => (release = r))));
  const h = createHistoryStore();
  await h.load("a");
  const p = h.load("b");
  expect(h.entries).toEqual([]);
  release([]);
  await p;
});
