import { listLiked, onLikedChanged, removeLiked, restoreLiked, toggleLike, type LikedTrack } from "./api";

/** Pure view of Rust's Liked tracks (newest first): replaced wholesale on every `liked-changed` event. */
export function createLikedStore() {
  let tracks: LikedTrack[] = $state([]);
  let started: Promise<void> | null = null;

  return {
    get tracks() { return tracks; },
    isLiked: (stationId: string, raw: string) => tracks.some((t) => t.stationId === stationId && t.raw === raw),
    /** Subscribe before the initial list to avoid missing an event. Idempotent: views remount. */
    init() {
      started ??= (async () => {
        const unlisten = await onLikedChanged((t) => (tracks = t));
        try {
          tracks = await listLiked();
        } catch (e) {
          unlisten();
          started = null;
          throw e;
        }
      })();
      return started;
    },
    toggle: (stationId: string, raw: string) => toggleLike(stationId, raw),
    remove: (stationId: string, raw: string) => removeLiked(stationId, raw),
    /** Puts a removed track back with its original `likedAt`. */
    restore: (track: LikedTrack) => restoreLiked(track),
  };
}

export const likedStore = createLikedStore();
