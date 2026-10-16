import { expect, it } from "vitest";
import { filterByGenre, genresOf } from "./genres";
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
