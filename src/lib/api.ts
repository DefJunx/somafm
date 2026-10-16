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
