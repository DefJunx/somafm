import { afterEach, expect, it } from "vitest";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { invoke } from "@tauri-apps/api/core";
import type { PlayerState } from "./api";

afterEach(() => clearMocks());

it("round-trips get_player_state through mockIPC", async () => {
  const state: PlayerState = {
    stationId: null,
    status: { kind: "stopped" },
    volume: 0.8,
    quality: "mp3",
    nowPlaying: null,
  };
  mockIPC((cmd) => (cmd === "get_player_state" ? state : undefined));
  expect(await invoke<PlayerState>("get_player_state")).toEqual(state);
});
