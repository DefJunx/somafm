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
