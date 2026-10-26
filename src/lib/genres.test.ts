import { expect, it } from "vitest";
import { HUES, filterByGenre, genresOf, hueOfGenre, hueVar, stationHue } from "./genres";
import { station } from "./fixtures";

const feed = [
  station("a", ["Ambient", "Chill"]),
  station("b", ["Rock"]),
  station("c", ["Chill", "Rock"]),
  station("d", []),
];

it("derives distinct genres in first-seen feed order", () => {
  expect(genresOf(feed)).toEqual(["Ambient", "Chill", "Rock"]);
});

it("returns all stations in feed order when no genre is selected", () => {
  expect(filterByGenre(feed, null).map((s) => s.id)).toEqual(["a", "b", "c", "d"]);
});

it("filters by genre preserving feed order", () => {
  expect(filterByGenre(feed, "Rock").map((s) => s.id)).toEqual(["b", "c"]);
});

it("maps a genre to a stable hue, ignoring case and spacing", () => {
  expect(HUES).toContain(hueOfGenre("Ambient"));
  expect(hueOfGenre("Ambient")).toBe(hueOfGenre("  ambient "));
  expect(hueOfGenre("Rock")).toBe(hueOfGenre("Rock"));
});

it("spreads a catalogue of genres over more than one hue", () => {
  const used = new Set(["Ambient", "Chill", "Rock", "Jazz", "Drone", "Metal", "Reggae", "Indie"].map(hueOfGenre));
  expect(used.size).toBeGreaterThan(2);
});

it("takes a Station's hue from its first genre, violet without genres", () => {
  expect(stationHue(station("x", ["Rock", "Ambient"]))).toBe(hueOfGenre("Rock"));
  expect(stationHue(station("y", []))).toBe("violet");
  expect(hueVar("cyan")).toBe("var(--hue-cyan)");
});
