import { expect, it } from "vitest";
import { filterLiked, groupLiked } from "./likedGroups";
import type { LikedTrack } from "./api";

const at = (d: number, h = 12, m = 0) => new Date(2026, 9, d, h, m).getTime(); // October 2026, local time
const now = at(21, 15);
const L = (raw: string, likedAt: number, stationId = "a"): LikedTrack => ({ stationId, raw, likedAt });

it("groups newest-first tracks into Today, Yesterday, This week and month sections", () => {
  const ts = [L("a", at(21, 9)), L("b", at(20, 23)), L("c", at(18)), L("d", at(15)), L("e", at(14)), L("f", new Date(2026, 8, 30, 12).getTime()), L("g", new Date(2025, 11, 1).getTime())];
  const g = groupLiked(ts, now, "en-US");
  expect(g.map((x) => x.label)).toEqual(["Today", "Yesterday", "This week", "October 2026", "September 2026", "December 2025"]);
  expect(g.map((x) => x.items.map((t) => t.raw))).toEqual([["a"], ["b"], ["c", "d"], ["e"], ["f"], ["g"]]);
  expect(g.map((x) => x.timeOnly)).toEqual([true, true, true, false, false, false]);
});

it("treats future timestamps as today and empty input as no groups", () => {
  expect(groupLiked([L("a", at(22))], now, "en-US")[0].label).toBe("Today");
  expect(groupLiked([], now, "en-US")).toEqual([]);
});

it("uses local midnight, not 24h windows, for Today and Yesterday", () => {
  const g = groupLiked([L("a", at(21, 0, 1)), L("b", at(20, 23, 59))], at(21, 0, 5), "en-US");
  expect(g.map((x) => x.label)).toEqual(["Today", "Yesterday"]);
});

const stations = new Map([["a", "Groove Salad"], ["b", "Drone Zone"]]);
const ts = [L("Massive Attack - Teardrop", 3), L("Solo", 2, "b"), L("Boards of Canada - Roygbiv", 1)];
it("filters case-insensitively on track text and station title; blank query keeps all", () => {
  const f = (q: string) => filterLiked(ts, q, (id) => stations.get(id)).map((t) => t.raw);
  expect(f("")).toHaveLength(3);
  expect(f("  ")).toHaveLength(3);
  expect(f("TEARDROP")).toEqual(["Massive Attack - Teardrop"]);
  expect(f("drone")).toEqual(["Solo"]);
  expect(f("groove")).toEqual(["Massive Attack - Teardrop", "Boards of Canada - Roygbiv"]);
  expect(f("nope")).toEqual([]);
});
