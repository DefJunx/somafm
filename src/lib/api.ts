import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Quality = "mp3" | "aac128";
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
  quality: Quality;
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
export const setQuality = (quality: Quality) => invoke<void>("set_quality", { quality });
export const onPlayerState = (cb: (s: PlayerState) => void): Promise<UnlistenFn> =>
  listen<PlayerState>("player-state", (e) => cb(e.payload));
