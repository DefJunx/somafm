import type { LikedTrack } from "./api";
import { rawText } from "./tracks";

export interface LikedGroup {
  /** Stable key for keyed rendering. */
  key: string;
  label: string;
  /** Rows show only the time of day (recent groups) rather than the day and month. */
  timeOnly: boolean;
  items: LikedTrack[];
}

const startOfDay = (ms: number) => { const d = new Date(ms); d.setHours(0, 0, 0, 0); return d; };

/** Groups newest-first tracks by local day relative to `now` (injected): Today, Yesterday, This week (2-6 days ago), then by month. */
export function groupLiked(tracks: LikedTrack[], now: number, locale?: string): LikedGroup[] {
  const today = startOfDay(now);
  const month = new Intl.DateTimeFormat(locale, { month: "long", year: "numeric" });
  const groups: LikedGroup[] = [];
  for (const t of tracks) {
    const d = startOfDay(t.likedAt);
    const days = Math.round((today.getTime() - d.getTime()) / 86_400_000);
    let key: string, label: string, timeOnly = true;
    if (days <= 0) { key = "today"; label = "Today"; }
    else if (days === 1) { key = "yesterday"; label = "Yesterday"; }
    else if (days < 7) { key = "week"; label = "This week"; }
    else { key = `${d.getFullYear()}-${d.getMonth()}`; label = month.format(d); timeOnly = false; }
    const last = groups[groups.length - 1];
    if (last?.key === key) last.items.push(t);
    else groups.push({ key, label, timeOnly, items: [t] });
  }
  return groups;
}

/** Case-insensitive match of `query` on the track text and the Station title. */
export function filterLiked(tracks: LikedTrack[], query: string, stationTitle: (id: string) => string | undefined): LikedTrack[] {
  const q = query.trim().toLowerCase();
  if (!q) return tracks;
  return tracks.filter((t) => rawText(t.raw).toLowerCase().includes(q) || t.raw.toLowerCase().includes(q) || (stationTitle(t.stationId) ?? "").toLowerCase().includes(q));
}
