import { getPlayerState, onPlayerState, play, setVolume, stop, type PlayerState } from "./api";

export const initialPlayerState: PlayerState = {
  stationId: null,
  status: { kind: "stopped" },
  volume: 0.8,
  nowPlaying: null,
};

/** Pure view of Rust's PlayerState: replaced wholesale on every `player-state` event. */
export function createPlayerStore() {
  let state: PlayerState = $state(initialPlayerState);
  let started: Promise<void> | null = null;

  return {
    get state() { return state; },
    /** Subscribe before the initial get to avoid missing an event. Idempotent: views remount. */
    init() {
      started ??= (async () => {
        const unlisten = await onPlayerState((s) => (state = s));
        try {
          state = await getPlayerState();
        } catch (e) {
          unlisten();
          started = null;
          throw e;
        }
      })();
      return started;
    },
    play: (stationId: string) => play(stationId),
    stop: () => stop(),
    setVolume: (v: number) => setVolume(Math.min(1, Math.max(0, v))),
  };
}

export const playerStore = createPlayerStore();

/** Status line text (spec §6). */
export function statusText(s: PlayerState): string {
  switch (s.status.kind) {
    case "connecting": return s.status.reconnecting ? "Reconnecting..." : "Connecting...";
    case "error": return s.status.message;
    case "stopped": return s.stationId ? "Paused" : "";
    case "playing": return "";
  }
}

/** Short Status name for the badge: Playing, Paused, Connecting, Reconnecting or Error. */
export function statusLabel(s: PlayerState): string {
  switch (s.status.kind) {
    case "connecting": return s.status.reconnecting ? "Reconnecting" : "Connecting";
    case "error": return "Error";
    case "stopped": return "Paused";
    case "playing": return "Playing";
  }
}

export function nowPlayingText(s: PlayerState, stationTitle: string): string {
  const np = s.nowPlaying;
  return np ? `${np.artist ?? stationTitle} - ${np.title}` : "";
}

export interface PrimaryAction { label: "Stop" | "Play" | "Retry"; busy: boolean }

/** The single transport button: Stop while playing or connecting (`busy` marks connecting; the bar keeps Stop enabled, Player shows a spinner), Retry on error, else Play. */
export function primaryAction(s: PlayerState): PrimaryAction {
  switch (s.status.kind) {
    case "playing": return { label: "Stop", busy: false };
    case "connecting": return { label: "Stop", busy: true };
    case "error": return { label: "Retry", busy: false };
    case "stopped": return { label: "Play", busy: false };
  }
}

/** Runs the primary action: Stop stops, anything else (re)plays the loaded station. */
export function runPrimaryAction(player: ReturnType<typeof createPlayerStore>) {
  const state = player.state;
  if (!state.stationId) return;
  if (primaryAction(state).label === "Stop") player.stop();
  else player.play(state.stationId);
}
