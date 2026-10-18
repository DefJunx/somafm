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

export interface SettingsView { favourites: string[]; favouritesOnly: boolean }
export const getSettings = () => invoke<SettingsView>("get_settings");
export const toggleFavourite = (stationId: string) => invoke<void>("toggle_favourite", { stationId });
export const setFavouritesOnly = (value: boolean) => invoke<void>("set_favourites_only", { value });
export const onSettingsChanged = (cb: (s: SettingsView) => void): Promise<UnlistenFn> =>
  listen<SettingsView>("settings-changed", (e) => cb(e.payload));
