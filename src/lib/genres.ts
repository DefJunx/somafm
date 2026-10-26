import type { Station } from "./api";

/** Distinct genres in first-seen feed order (stable across refreshes). */
export function genresOf(stations: Station[]): string[] {
  const seen = new Set<string>();
  for (const s of stations) for (const g of s.genres) seen.add(g);
  return [...seen];
}

/** `null` means the All chip. Feed order is preserved. */
export function filterByGenre(stations: Station[], genre: string | null): Station[] {
  return genre === null ? stations : stations.filter((s) => s.genres.includes(genre));
}

/** Favourites present in the feed, in feed order. Ids missing from the feed are hidden (never pruned). */
export function favouritesIn(stations: Station[], favourites: string[]): Station[] {
  const set = new Set(favourites);
  return stations.filter((s) => set.has(s.id));
}

export const HUES = ["violet", "indigo", "cyan", "green", "orange", "red"] as const;
export type Hue = (typeof HUES)[number];

/** Stable genre -> hue (FNV-1a over the lower-cased name), so a Station keeps its hue across sessions and feeds. */
export function hueOfGenre(genre: string): Hue {
  let h = 0x811c9dc5;
  for (const ch of genre.trim().toLowerCase()) {
    h ^= ch.codePointAt(0)!;
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return HUES[h % HUES.length];
}

/** A Station's hue comes from its first genre; a Station without genres is violet. */
export function stationHue(station: Pick<Station, "genres">): Hue {
  const first = station.genres[0];
  return first ? hueOfGenre(first) : "violet";
}

/** CSS value for a hue token, for `style:--hue`. */
export const hueVar = (hue: Hue) => `var(--hue-${hue})`;

/** CSS value for a Station's hue (or the dim ink while the Station is unknown). */
export const stationHueVar = (station: Pick<Station, "genres"> | undefined) => (station ? hueVar(stationHue(station)) : "var(--ink-dim)");
