import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type ErrorKind = "offline" | "unavailable" | "unsupported";
export type Status =
  | { kind: "stopped" }
  | { kind: "connecting"; reconnecting: boolean }
  | { kind: "playing" }
  | { kind: "error"; errorKind: ErrorKind; message: string };
export interface NowPlaying { raw: string; artist: string | null; title: string }
export interface PlayerState {
  stationId: string | null;
  status: Status;
  volume: number;
  nowPlaying: NowPlaying | null;
}

export interface Station {
  id: string; title: string; description: string; dj: string;
  genres: string[];          // `genre` split on "|"
  listeners: number;         // feed string parsed; 0 if unparsable
  lastPlaying: string;
  image: string; largeImage: string; xlImage: string; // feed image/largeimage/xlimage URLs
}
export interface StationsSnapshot { stations: Station[]; loaded: boolean } // loaded=false: no cache and no successful fetch yet

export const getStations = () => invoke<StationsSnapshot>("get_stations");
export const refreshStations = () => invoke<StationsSnapshot>("refresh_stations");
export const onStationsUpdated = (cb: (s: StationsSnapshot) => void): Promise<UnlistenFn> =>
  listen<StationsSnapshot>("stations-updated", (e) => cb(e.payload));

export const getPlayerState = () => invoke<PlayerState>("get_player_state");
export const play = (stationId: string) => invoke<void>("play", { stationId });
export const stop = () => invoke<void>("stop");
export const setVolume = (volume: number) => invoke<void>("set_volume", { volume });
export const onPlayerState = (cb: (s: PlayerState) => void): Promise<UnlistenFn> =>
  listen<PlayerState>("player-state", (e) => cb(e.payload));

export interface SettingsView { favourites: string[]; favouritesOnly: boolean; fadeAudio: boolean; showVisualizer: boolean }
export const getSettings = () => invoke<SettingsView>("get_settings");
export const toggleFavourite = (stationId: string) => invoke<void>("toggle_favourite", { stationId });
export const setFavouritesOnly = (value: boolean) => invoke<void>("set_favourites_only", { value });
export const setFadeAudio = (value: boolean) => invoke<void>("set_fade_audio", { value });
export const setShowVisualizer = (value: boolean) => invoke<void>("set_show_visualizer", { value });
/** The Player screen is (not) showing the Visualizer; Rust emits `spectrum` events only while it is. */
export const setVisualizerActive = (active: boolean) => invoke<void>("set_visualizer_active", { active });
/** 32 band magnitudes (0..1, lowest frequency first), about 30 per second. */
export const onSpectrum = (cb: (bands: number[]) => void): Promise<UnlistenFn> =>
  listen<number[]>("spectrum", (e) => cb(e.payload));
/** Reports the OS reduced-motion preference; Rust applies it only when settings.json was just created or migrated. */
export const applyReducedMotionDefault = (reduced: boolean) => invoke<void>("apply_reduced_motion_default", { reduced });
export const onSettingsChanged = (cb: (s: SettingsView) => void): Promise<UnlistenFn> =>
  listen<SettingsView>("settings-changed", (e) => cb(e.payload));

export interface HistoryEntry { time: number; artist: string; title: string; raw: string } // time: Unix seconds; raw: like key (= ICY raw)
export const getStationHistory = (stationId: string) =>
  invoke<HistoryEntry[]>("get_station_history", { stationId });

export interface LikedTrack { raw: string; stationId: string; likedAt: number } // likedAt: Unix ms
export const listLiked = () => invoke<LikedTrack[]>("list_liked"); // newest first
export const toggleLike = (stationId: string, raw: string) =>
  invoke<boolean>("toggle_like", { stationId, raw }); // resolves to liked-after
export const removeLiked = (stationId: string, raw: string) =>
  invoke<void>("remove_liked", { stationId, raw });
export const restoreLiked = (track: LikedTrack) => invoke<void>("restore_liked", { track }); // undo of a removal
export const onLikedChanged = (cb: (t: LikedTrack[]) => void): Promise<UnlistenFn> =>
  listen<LikedTrack[]>("liked-changed", (e) => cb(e.payload));

// Needs tauri-plugin-opener registered in Rust plus the `opener:allow-open-url` capability.
export const openUrl = (url: string) => invoke<void>("plugin:opener|open_url", { url });
