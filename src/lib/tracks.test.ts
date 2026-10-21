import { expect, it } from "vitest";
import { rawText, searchLinks, splitRaw, trackText } from "./tracks";

it("splits artist and title on the first ' - ' only", () => {
  expect(splitRaw("Boards of Canada - Roygbiv")).toEqual({ artist: "Boards of Canada", title: "Roygbiv" });
  expect(splitRaw("A - B - C")).toEqual({ artist: "A", title: "B - C" });
});

it("has no artist when the raw string has no ' - '", () => {
  expect(splitRaw("Just a title")).toEqual({ artist: null, title: "Just a title" });
  expect(splitRaw("a-b")).toEqual({ artist: null, title: "a-b" });
});

it("trims both sides, and a blank artist counts as none", () => {
  expect(splitRaw("  A  -  B ")).toEqual({ artist: "A", title: "B" });
  expect(splitRaw("  - Title")).toEqual({ artist: null, title: "Title" });
});

it("displays 'artist – title', or just the title for a blank artist", () => {
  expect(trackText("X", "Y")).toBe("X – Y");
  expect(trackText("", "Y")).toBe("Y");
  expect(trackText(null, "Y")).toBe("Y");
  expect(rawText("A - B - C")).toBe("A – B - C");
  expect(rawText("Solo")).toBe("Solo");
});

it("builds URL-encoded search links for Bandcamp, Discogs and MusicBrainz", () => {
  const links = searchLinks("AC/DC - Back & Black");
  expect(links.map((l) => l.name)).toEqual(["Bandcamp", "Discogs", "MusicBrainz"]);
  expect(links[0].url).toBe("https://bandcamp.com/search?q=AC%2FDC%20Back%20%26%20Black");
  expect(links[1].url).toBe("https://www.discogs.com/search/?q=AC%2FDC%20Back%20%26%20Black&type=all");
  expect(links[2].url).toBe("https://musicbrainz.org/search?query=AC%2FDC%20Back%20%26%20Black&type=recording");
});

it("searches the whole raw string when it has no split, and keeps extra dashes", () => {
  expect(searchLinks("Solo")[0].url).toBe("https://bandcamp.com/search?q=Solo");
  expect(searchLinks("A - B - C")[0].url).toBe("https://bandcamp.com/search?q=A%20B%20-%20C");
});
