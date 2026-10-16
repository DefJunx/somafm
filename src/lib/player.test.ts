import { afterEach, expect, it } from "vitest";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import { createPlayerStore, initialPlayerState, nowPlayingText, statusText } from "./player.svelte";
import type { PlayerState } from "./api";

afterEach(() => clearMocks());

const st = (over: Partial<PlayerState>): PlayerState => ({ ...initialPlayerState, ...over });

it("maps get_player_state, then replaces state on each player-state event", async () => {
  mockIPC((cmd) => (cmd === "get_player_state" ? st({ stationId: "a" }) : undefined), { shouldMockEvents: true });
  const p = createPlayerStore();
  await p.init();
  expect(p.state.stationId).toBe("a");
  await emit("player-state", st({ stationId: "b", status: { kind: "playing" } }));
  await Promise.resolve();
  expect(p.state.stationId).toBe("b");
  expect(p.state.status.kind).toBe("playing");
});

it("sends commands with camelCase args and clamps volume", async () => {
  const calls: [string, unknown][] = [];
  mockIPC((cmd, args) => { calls.push([cmd, args]); }, { shouldMockEvents: true });
  const p = createPlayerStore();
  await p.play("groovesalad");
  await p.stop();
  await p.setVolume(3);
  await p.setQuality("aac128");
  expect(calls).toEqual([
    ["play", { stationId: "groovesalad" }],
    ["stop", {}],
    ["set_volume", { volume: 1 }],
    ["set_quality", { quality: "aac128" }],
  ]);
});

it("renders status and now-playing text", () => {
  expect(statusText(st({}))).toBe("");
  expect(statusText(st({ stationId: "a" }))).toBe("Paused");
  expect(statusText(st({ status: { kind: "connecting", reconnecting: false } }))).toBe("Connecting...");
  expect(statusText(st({ status: { kind: "connecting", reconnecting: true } }))).toBe("Reconnecting...");
  expect(statusText(st({ status: { kind: "error", errorKind: "offline", message: "You're offline" } }))).toBe("You're offline");
  expect(nowPlayingText(st({}), "S")).toBe("");
  expect(nowPlayingText(st({ nowPlaying: { raw: "x - y", artist: "x", title: "y" } }), "S")).toBe("x - y");
  expect(nowPlayingText(st({ nowPlaying: { raw: "y", artist: null, title: "y" } }), "S")).toBe("S - y");
});
