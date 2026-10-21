/** Splits on the first " - " and trims both sides (like `icy::now_playing`); no artist when there is none or it is blank. */
export function splitRaw(raw: string): { artist: string | null; title: string } {
  const i = raw.indexOf(" - ");
  if (i < 0) return { artist: null, title: raw.trim() };
  return { artist: raw.slice(0, i).trim() || null, title: raw.slice(i + 3).trim() };
}

/** The one display form of a track: "artist – title", or just the title when the artist is blank. */
export const trackText = (artist: string | null, title: string) => (artist ? `${artist} – ${title}` : title);

/** Display form of a stored raw string. */
export function rawText(raw: string): string {
  const { artist, title } = splitRaw(raw);
  return trackText(artist, title);
}

/** "Search on" links; the query is "artist title" when the raw string splits, else the raw string. */
export function searchLinks(raw: string): { name: string; url: string }[] {
  const { artist, title } = splitRaw(raw);
  const q = encodeURIComponent(artist === null ? title : `${artist} ${title}`);
  return [
    { name: "Bandcamp", url: `https://bandcamp.com/search?q=${q}` },
    { name: "Discogs", url: `https://www.discogs.com/search/?q=${q}&type=all` },
    { name: "MusicBrainz", url: `https://musicbrainz.org/search?query=${q}&type=recording` },
  ];
}
