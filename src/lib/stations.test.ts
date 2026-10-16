import { afterEach, expect, it } from "vitest";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import { createStationsStore } from "./stations.svelte";
import { station } from "./fixtures";
import type { StationsSnapshot } from "./api";

afterEach(() => clearMocks());

const snap = (ids: string[], loaded = true): StationsSnapshot => ({
  stations: ids.map((i) => station(i)),
  loaded,
});

it("starts unloaded and empty before init resolves", () => {
  mockIPC(() => snap([]), { shouldMockEvents: true });
  const s = createStationsStore();
  expect(s.loaded).toBe(false);
  expect(s.stations).toEqual([]);
});

it("maps the get_stations result into state", async () => {
  mockIPC((cmd) => (cmd === "get_stations" ? snap(["a", "b"]) : undefined), { shouldMockEvents: true });
  const s = createStationsStore();
  await s.init();
  expect(s.loaded).toBe(true);
  expect(s.stations.map((x) => x.id)).toEqual(["a", "b"]);
});

it("reports offline (loaded=false) when Rust has no data", async () => {
  mockIPC(() => snap([], false), { shouldMockEvents: true });
  const s = createStationsStore();
  await s.init();
  expect(s.loaded).toBe(false);
});

it("replaces state wholesale on stations-updated", async () => {
  mockIPC(() => snap([], false), { shouldMockEvents: true });
  const s = createStationsStore();
  await s.init();
  await emit("stations-updated", snap(["x", "y"]));
  expect(s.loaded).toBe(true);
  expect(s.stations.map((x) => x.id)).toEqual(["x", "y"]);
});

it("retry calls refresh_stations and applies the result", async () => {
  const calls: string[] = [];
  mockIPC((cmd) => {
    calls.push(cmd);
    return cmd === "refresh_stations" ? snap(["r"]) : snap([], false);
  }, { shouldMockEvents: true });
  const s = createStationsStore();
  await s.init();
  await s.retry();
  expect(calls).toContain("refresh_stations");
  expect(s.stations.map((x) => x.id)).toEqual(["r"]);
});

it("keeps current state if refresh_stations rejects", async () => {
  mockIPC((cmd) => {
    if (cmd === "refresh_stations") throw "boom";
    return snap(["a"]);
  }, { shouldMockEvents: true });
  const s = createStationsStore();
  await s.init();
  await s.retry();
  expect(s.stations.map((x) => x.id)).toEqual(["a"]);
});

it("is not ready until the first snapshot is applied", async () => {
  mockIPC(() => snap(["a"]), { shouldMockEvents: true });
  const s = createStationsStore();
  expect(s.ready).toBe(false);
  await s.init();
  expect(s.ready).toBe(true);
});

it("init rejection resets so a later init retries", async () => {
  let fail = true;
  mockIPC((cmd) => {
    if (cmd === "get_stations" && fail) throw "boom";
    return snap(["a"]);
  }, { shouldMockEvents: true });
  const s = createStationsStore();
  await expect(s.init()).rejects.toBeTruthy();
  expect(s.ready).toBe(false);
  fail = false;
  await s.init();
  expect(s.stations.map((x) => x.id)).toEqual(["a"]);
});
