import { afterEach, expect, it } from "vitest";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { createHistoryStore } from "./history.svelte";
import type { HistoryEntry } from "./api";

afterEach(() => clearMocks());

const e = (artist: string, title: string): HistoryEntry => ({ time: 1_700_000_000, artist, title, raw: `${artist} - ${title}` });
const tick = () => new Promise((resolve) => setTimeout(resolve, 0));

it("loads the history for a station", async () => {
  mockIPC((cmd, args) => (cmd === "get_station_history" ? [e((args as { stationId: string }).stationId, "t")] : undefined));
  const h = createHistoryStore();
  h.createSession().update(true, "a", null);
  await tick();
  expect(h.entries).toEqual([e("a", "t")]);
  expect(h.failed).toBe(false);
});

it("flags a failure without retrying, and a later load can recover", async () => {
  let calls = 0;
  mockIPC(() => { calls++; if (calls === 1) throw new Error("boom"); return [e("x", "y")]; });
  const h = createHistoryStore();
  const session = h.createSession();
  session.update(true, "a", null);
  await tick();
  expect(h.failed).toBe(true);
  expect(h.entries).toEqual([]);
  expect(calls).toBe(1);
  session.update(true, "a", "next");
  await tick();
  expect(h.failed).toBe(false);
  expect(h.entries).toHaveLength(1);
});

it("discards the previous list on a station switch and ignores a stale response", async () => {
  const pending: Record<string, (v: HistoryEntry[]) => void> = {};
  mockIPC((cmd, args) => new Promise((r) => (pending[(args as { stationId: string }).stationId] = r)));
  const h = createHistoryStore();
  const session = h.createSession();
  session.update(true, "a", null);
  session.update(true, "b", null);
  pending.b([e("b", "bt")]);
  await tick();
  pending.a([e("a", "at")]);
  await tick();
  expect(h.entries).toEqual([e("b", "bt")]);
});

it("clears entries immediately when the station changes", async () => {
  let release: (v: HistoryEntry[]) => void = () => {};
  let n = 0;
  mockIPC(() => (n++ === 0 ? [e("a", "t")] : new Promise((r) => (release = r))));
  const h = createHistoryStore();
  const session = h.createSession();
  session.update(true, "a", null);
  await tick();
  session.update(true, "b", null);
  expect(h.entries).toEqual([]);
  release([]);
  await tick();
});

it("keeps refresh eligibility per Player session while requests stay ordered in the store", async () => {
  const pending: ((v: HistoryEntry[]) => void)[] = [];
  mockIPC(() => new Promise((r) => pending.push(r)));
  const h = createHistoryStore();
  const first = h.createSession();
  first.update(true, "a", null);
  first.update(true, "a", null);
  first.update(false, "a", "x");
  first.update(false, "a", "y");
  first.update(true, "a", "y");
  expect(pending).toHaveLength(2);

  const reopened = h.createSession();
  reopened.update(true, "a", null);
  expect(pending).toHaveLength(3);

  pending[0]([e("old", "stale")]);
  await tick();
  expect(h.entries).toEqual([]);
  pending[2]([e("a", "latest")]);
  await tick();
  expect(h.entries).toEqual([e("a", "latest")]);
});

it("lets a request finish after its Player session ends", async () => {
  let release: (v: HistoryEntry[]) => void = () => {};
  mockIPC(() => new Promise((r) => (release = r)));
  const h = createHistoryStore();
  const session = h.createSession();
  session.update(true, "a", "x");
  session.update(false, "a", "x");
  release([e("a", "finished")]);
  await tick();
  expect(h.entries).toEqual([e("a", "finished")]);
});
