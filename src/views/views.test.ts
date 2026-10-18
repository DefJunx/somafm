import { afterEach, expect, it } from "vitest";
import { render, screen, cleanup, fireEvent, waitFor } from "@testing-library/svelte";
import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import Grid from "./Grid.svelte";
import Player from "./Player.svelte";
import { createStationsStore } from "../lib/stations.svelte";
import { createPlayerStore, initialPlayerState } from "../lib/player.svelte";
import { createSettingsStore } from "../lib/settings.svelte";
import { station } from "../lib/fixtures";
import type { PlayerState, SettingsView, StationsSnapshot } from "../lib/api";

afterEach(() => { cleanup(); clearMocks(); });

const feed: StationsSnapshot = {
  loaded: true,
  stations: [station("a", ["Ambient"]), station("b", ["Rock"]), station("c", ["Ambient"])],
};

const ps = (over: Partial<PlayerState> = {}): PlayerState => ({ ...initialPlayerState, ...over });
type Call = [string, unknown];

/** Routes IPC by command: player state / stations / records player commands. */
function ipc(opts: { snap?: StationsSnapshot; player?: PlayerState; settings?: SettingsView; calls?: Call[] } = {}) {
  mockIPC(
    (cmd, args) => {
      if (cmd === "get_settings") return opts.settings ?? { favourites: [], favouritesOnly: false };
      if (cmd === "get_player_state") return opts.player ?? ps();
      if (cmd === "get_stations" || cmd === "refresh_stations") return opts.snap ?? feed;
      opts.calls?.push([cmd, args]);
    },
    { shouldMockEvents: true },
  );
}

it("shows Tiles in feed order and filters by genre chip", async () => {
  ipc();
  render(Grid, { store: createStationsStore(), player: createPlayerStore() });
  await waitFor(() => expect(screen.getAllByRole("img")).toHaveLength(3));
  const ambient = screen.getByRole("button", { name: "Ambient" });
  await fireEvent.click(ambient);
  expect(ambient.getAttribute("aria-pressed")).toBe("true");
  expect(screen.getAllByRole("img").map((i) => i.getAttribute("alt"))).toEqual(["Station a", "Station c"]);
  await fireEvent.click(screen.getByRole("button", { name: "All" }));
  expect(screen.getAllByRole("img")).toHaveLength(3);
});

it("falls back to the station name when an image fails", async () => {
  ipc();
  render(Grid, { store: createStationsStore(), player: createPlayerStore() });
  const imgs = await waitFor(() => screen.getAllByRole("img"));
  await fireEvent.error(imgs[0]);
  expect(screen.getAllByRole("img")).toHaveLength(2);
  expect(screen.getByText("Station a")).toBeTruthy();
});

it("shows the empty state and Retry refreshes", async () => {
  mockIPC((cmd) => (cmd === "get_player_state" ? ps() : cmd === "refresh_stations" ? feed : { stations: [], loaded: false }), { shouldMockEvents: true });
  render(Grid, { store: createStationsStore(), player: createPlayerStore() });
  await screen.findByText("Can't reach SomaFM");
  await fireEvent.click(screen.getByRole("button", { name: "Retry" }));
  await waitFor(() => expect(screen.getAllByRole("img")).toHaveLength(3));
});

it("does not flash the empty state before the first snapshot", async () => {
  mockIPC((cmd) => (cmd === "get_player_state" ? ps() : new Promise(() => {})), { shouldMockEvents: true });
  render(Grid, { store: createStationsStore(), player: createPlayerStore() });
  await Promise.resolve();
  expect(screen.queryByText("Can't reach SomaFM")).toBeNull();
});

it("retries the image fallback when the image URL changes", async () => {
  const store = createStationsStore();
  ipc();
  render(Grid, { store, player: createPlayerStore() });
  const imgs = await waitFor(() => screen.getAllByRole("img"));
  await fireEvent.error(imgs[0]);
  expect(screen.getAllByRole("img")).toHaveLength(2);
  await emit("stations-updated", {
    loaded: true,
    stations: [station("a", ["Ambient"], { largeImage: "https://img.example/new.png" }), ...feed.stations.slice(1)],
  });
  await waitFor(() => expect(screen.getAllByRole("img")).toHaveLength(3));
});

it("clicking a Tile plays that Station and opens the Player screen", async () => {
  const calls: Call[] = [];
  let opened = 0;
  ipc({ calls });
  render(Grid, { store: createStationsStore(), player: createPlayerStore(), onopenplayer: () => opened++ });
  await fireEvent.click(await screen.findByRole("button", { name: "Play Station b" }));
  expect(calls).toContainEqual(["play", { stationId: "b" }]);
  expect(opened).toBe(1);
});

it("marks only the active Tile and the badge follows the status", async () => {
  ipc({ player: ps({ stationId: "b", status: { kind: "playing" } }) });
  render(Grid, { store: createStationsStore(), player: createPlayerStore() });
  const tile = await screen.findByRole("button", { name: "Play Station b" });
  await waitFor(() => expect(tile.getAttribute("aria-current")).toBe("true"));
  expect(screen.getByRole("button", { name: "Play Station a" }).getAttribute("aria-current")).toBeNull();
  expect(screen.getAllByTestId("badge")).toHaveLength(1);
  expect(screen.getByTestId("badge").getAttribute("data-status")).toBe("playing");

  expect(document.querySelector(".eq")).toBeTruthy();
  await emit("player-state", ps({ stationId: "b", status: { kind: "stopped" } }));
  await waitFor(() => expect(screen.getByTestId("badge").getAttribute("data-status")).toBe("stopped"));
  expect(document.querySelector(".eq")).toBeNull(); // Paused: ring + static badge, no equaliser
  expect(tile.getAttribute("aria-current")).toBe("true");
  await emit("player-state", ps({ stationId: "b", status: { kind: "connecting", reconnecting: false } }));
  await waitFor(() => expect(screen.getByTestId("badge").getAttribute("data-status")).toBe("connecting"));
  expect(document.querySelector(".eq")).toBeNull();
  await emit("player-state", ps({ stationId: "b", status: { kind: "error", errorKind: "offline", message: "You're offline" } }));
  await waitFor(() => expect(screen.getByTestId("badge").getAttribute("data-status")).toBe("error"));
  expect(tile.getAttribute("aria-current")).toBe("true"); // ring stays in error
});

it("bottom bar shows the Station with Stop and a jump to the Player screen", async () => {
  const calls: Call[] = [];
  let opened = 0;
  ipc({ player: ps({ stationId: "a", status: { kind: "playing" } }), calls });
  render(Grid, { store: createStationsStore(), player: createPlayerStore(), onopenplayer: () => opened++ });
  const bar = await screen.findByRole("complementary", { name: "Now playing" });
  expect(bar.textContent).toContain("Station a");
  await fireEvent.click(screen.getByRole("button", { name: "Stop" }));
  expect(calls).toContainEqual(["stop", {}]);
  await fireEvent.click(screen.getByRole("button", { name: "Open player for Station a" }));
  expect(opened).toBe(1);
});

it("bottom bar is hidden with no station and offers Play when paused, Retry on error", async () => {
  const calls: Call[] = [];
  ipc({ calls });
  render(Grid, { store: createStationsStore(), player: createPlayerStore() });
  await screen.findByRole("button", { name: "Play Station a" });
  expect(screen.queryByRole("complementary")).toBeNull();
  await emit("player-state", ps({ stationId: "a" }));
  const bar = await screen.findByRole("complementary");
  expect(bar.textContent).toContain("Paused");
  await fireEvent.click(screen.getByRole("button", { name: "Play" }));
  expect(calls).toContainEqual(["play", { stationId: "a" }]);
  await emit("player-state", ps({ stationId: "a", status: { kind: "error", errorKind: "unavailable", message: "Station unavailable" } }));
  await screen.findByRole("button", { name: "Retry" });
  expect(screen.getByRole("complementary").textContent).toContain("Station unavailable");
});

async function renderPlayer(player: PlayerState, calls: Call[] = [], onback?: () => void) {
  ipc({ player, calls });
  const store = createPlayerStore();
  render(Player, { player: store, stations: createStationsStore(), onback });
  await screen.findByRole("heading", { name: /Station|Paused/ });
  return store;
}

it("Player screen shows the Station details and Stop while playing", async () => {
  const calls: Call[] = [];
  const snap = { loaded: true, stations: [station("a", [], { description: "Chill beats", listeners: 42 })] };
  ipc({ snap, player: ps({ stationId: "a", status: { kind: "playing" }, nowPlaying: { raw: "X - Y", artist: "X", title: "Y" } }), calls });
  render(Player, { player: createPlayerStore(), stations: createStationsStore() });
  await screen.findByRole("heading", { name: "Station a" });
  await screen.findByText("Chill beats");
  expect(screen.getByText("42 listeners")).toBeTruthy();
  expect(screen.getByText("X - Y")).toBeTruthy();
  expect(screen.getByRole("img", { name: "Station a" })).toBeTruthy();
  await fireEvent.click(screen.getByRole("button", { name: "Stop" }));
  expect(calls).toContainEqual(["stop", {}]);
});

it("Player screen status line and main button per status", async () => {
  const store = await renderPlayer(ps({ stationId: "a" }));
  expect(screen.getByRole("status").textContent).toBe("Paused");
  expect(screen.getByRole("button", { name: "Play" })).toBeTruthy();
  await emit("player-state", ps({ stationId: "a", status: { kind: "connecting", reconnecting: false } }));
  await waitFor(() => expect(screen.getByRole("status").textContent).toBe("Connecting..."));
  expect((screen.getByRole("button", { name: "Connecting" }) as HTMLButtonElement).disabled).toBe(true);
  await emit("player-state", ps({ stationId: "a", status: { kind: "connecting", reconnecting: true } }));
  await waitFor(() => expect(screen.getByRole("status").textContent).toBe("Reconnecting..."));
  await emit("player-state", ps({ stationId: "a", status: { kind: "error", errorKind: "offline", message: "You're offline" } }));
  await waitFor(() => expect(screen.getByRole("status").textContent).toBe("You're offline"));
  expect(screen.getByRole("button", { name: "Retry" })).toBeTruthy();
  expect(store.state.status.kind).toBe("error");
});

it("Player screen: Play/Retry plays, volume slider, mute and Back", async () => {
  const calls: Call[] = [];
  let backed = 0;
  await renderPlayer(ps({ stationId: "a", volume: 0.6 }), calls, () => backed++);
  await fireEvent.click(screen.getByRole("button", { name: "Play" }));
  expect(calls).toContainEqual(["play", { stationId: "a" }]);

  const slider = screen.getByLabelText("Volume") as HTMLInputElement;
  await fireEvent.input(slider, { target: { value: "0.3" } });
  expect(calls).toContainEqual(["set_volume", { volume: 0.3 }]);

  await fireEvent.click(screen.getByRole("button", { name: "Mute" }));
  expect(calls).toContainEqual(["set_volume", { volume: 0 }]);
  await emit("player-state", ps({ stationId: "a", volume: 0 }));
  await fireEvent.click(await screen.findByRole("button", { name: "Unmute" }));
  expect(calls.filter(([c]) => c === "set_volume").at(-1)).toEqual(["set_volume", { volume: 0.6 }]);

  expect(screen.queryByLabelText("Quality")).toBeNull();

  await fireEvent.click(screen.getByRole("button", { name: "Back" }));
  expect(backed).toBe(1);
});

it("bottom bar keeps an enabled Stop while connecting", async () => {
  const calls: Call[] = [];
  ipc({ player: ps({ stationId: "a", status: { kind: "connecting", reconnecting: false } }), calls });
  render(Grid, { store: createStationsStore(), player: createPlayerStore() });
  const stop = (await screen.findByRole("button", { name: "Stop" })) as HTMLButtonElement;
  expect(stop.disabled).toBe(false);
  await fireEvent.click(stop);
  expect(calls).toContainEqual(["stop", {}]);
});

const np = (artist: string | null, title: string) => ({ raw: artist ? `${artist} - ${title}` : title, artist, title });

it("bottom bar shows artist - title, station name as artist when none, and clears", async () => {
  ipc({ player: ps({ stationId: "a", status: { kind: "playing" }, nowPlaying: np("X", "Y") }) });
  render(Grid, { store: createStationsStore(), player: createPlayerStore() });
  const bar = await screen.findByRole("complementary", { name: "Now playing" });
  expect(bar.textContent).toContain("X - Y");
  await emit("player-state", ps({ stationId: "a", status: { kind: "playing" }, nowPlaying: np(null, "Solo") }));
  await waitFor(() => expect(bar.textContent).toContain("Station a - Solo"));
  await emit("player-state", ps({ stationId: "a", status: { kind: "stopped" }, nowPlaying: null }));
  await waitFor(() => expect(bar.textContent).toContain("Paused"));
  expect(bar.textContent).not.toContain("Solo");
});

it("Player screen shows both now-playing shapes and clears it", async () => {
  const store = await renderPlayer(ps({ stationId: "a", status: { kind: "playing" }, nowPlaying: np("X", "Y") }));
  expect(screen.getByText("X - Y")).toBeTruthy();
  await emit("player-state", ps({ stationId: "a", status: { kind: "playing" }, nowPlaying: np(null, "Solo") }));
  await waitFor(() => expect(screen.getByText("Station a - Solo")).toBeTruthy());
  await emit("player-state", ps({ stationId: "a", status: { kind: "stopped" }, nowPlaying: null }));
  await waitFor(() => expect(screen.queryByText("Station a - Solo")).toBeNull());
  expect(document.querySelector(".now")?.textContent).toBe("");
  expect(store.state.nowPlaying).toBeNull();
});

const errState = (errorKind: "offline" | "unavailable" | "unsupported", message: string) =>
  ps({ stationId: "a", status: { kind: "error", errorKind, message } });

const errorMessages = [
  ["offline", "You're offline"],
  ["unavailable", "Station unavailable"],
  ["unsupported", "Can't play this stream"],
] as const;

it("Player screen shows the message per error kind and Retry plays the loaded Station", async () => {
  const calls: Call[] = [];
  await renderPlayer(errState("offline", "You're offline"), calls);
  for (const [kind, message] of errorMessages) {
    await emit("player-state", errState(kind, message));
    await waitFor(() => expect(screen.getByRole("status").textContent).toBe(message));
    expect(document.querySelector(".spinner")).toBeNull();
  }
  await fireEvent.click(screen.getByRole("button", { name: "Retry" }));
  expect(calls).toContainEqual(["play", { stationId: "a" }]);
});

it("Player screen shows a spinner while connecting and reconnecting, none otherwise", async () => {
  await renderPlayer(ps({ stationId: "a", status: { kind: "connecting", reconnecting: false } }));
  expect(document.querySelector(".spinner")).toBeTruthy();
  await emit("player-state", ps({ stationId: "a", status: { kind: "connecting", reconnecting: true } }));
  await waitFor(() => expect(screen.getByRole("status").textContent).toBe("Reconnecting..."));
  expect(document.querySelector(".spinner")).toBeTruthy();
  await emit("player-state", ps({ stationId: "a", status: { kind: "playing" } }));
  await waitFor(() => expect(document.querySelector(".spinner")).toBeNull());
});

it("bottom bar shows status text per state, even when a track is kept, and Retry plays", async () => {
  const calls: Call[] = [];
  const kept = { raw: "X - Y", artist: "X", title: "Y" };
  ipc({ player: ps({ stationId: "a", status: { kind: "playing" }, nowPlaying: kept }), calls });
  render(Grid, { store: createStationsStore(), player: createPlayerStore() });
  const bar = await screen.findByRole("complementary", { name: "Now playing" });
  expect(bar.textContent).toContain("X - Y");
  await emit("player-state", ps({ stationId: "a", status: { kind: "connecting", reconnecting: true }, nowPlaying: kept }));
  await waitFor(() => expect(bar.textContent).toContain("Reconnecting..."));
  expect(bar.textContent).not.toContain("X - Y");
  await emit("player-state", ps({ stationId: "a", status: { kind: "connecting", reconnecting: false } }));
  await waitFor(() => expect(bar.textContent).toContain("Connecting..."));
  for (const [kind, message] of errorMessages) {
    await emit("player-state", { ...errState(kind, message), nowPlaying: kept });
    await waitFor(() => expect(bar.textContent).toContain(message));
  }
  await fireEvent.click(screen.getByRole("button", { name: "Retry" }));
  expect(calls).toContainEqual(["play", { stationId: "a" }]);
});

it("Grid badge pulses while connecting or reconnecting, warns in error, ring stays", async () => {
  ipc({ player: ps({ stationId: "b", status: { kind: "connecting", reconnecting: true } }) });
  render(Grid, { store: createStationsStore(), player: createPlayerStore() });
  const tile = await screen.findByRole("button", { name: "Play Station b" });
  await waitFor(() => expect(screen.getByTestId("badge").getAttribute("data-status")).toBe("connecting"));
  expect(document.querySelector(".badge .dot:not(.still)")).toBeTruthy();
  await emit("player-state", errState("unavailable", "Station unavailable"));
  await emit("player-state", ps({ stationId: "b", status: { kind: "error", errorKind: "unsupported", message: "Can't play this stream" } }));
  await waitFor(() => expect(screen.getByTestId("badge").querySelector("svg")).toBeTruthy());
  expect(document.querySelector(".badge .dot")).toBeNull();
  expect(tile.getAttribute("aria-current")).toBe("true");
});

const names = () => screen.getAllByRole("img").map((i) => i.getAttribute("alt"));

it("Favourites chip shows only Favourites present in the feed, in feed order", async () => {
  const calls: Call[] = [];
  ipc({ settings: { favourites: ["c", "gone", "a"], favouritesOnly: false }, calls });
  render(Grid, { store: createStationsStore(), player: createPlayerStore(), settings: createSettingsStore() });
  await waitFor(() => expect(screen.getAllByRole("img")).toHaveLength(3));
  const chip = screen.getByRole("button", { name: "Favourites" });
  expect(chip.getAttribute("aria-pressed")).toBe("false");
  await fireEvent.click(chip);
  expect(calls).toContainEqual(["set_favourites_only", { value: true }]);
  // Rust confirms with settings-changed; the view follows the event.
  await emit("settings-changed", { favourites: ["c", "gone", "a"], favouritesOnly: true });
  await waitFor(() => expect(names()).toEqual(["Station a", "Station c"]));
  expect(chip.getAttribute("aria-pressed")).toBe("true");
  await fireEvent.click(screen.getByRole("button", { name: "All" }));
  expect(calls).toContainEqual(["set_favourites_only", { value: false }]);
});

it("restores the persisted Favourites chip state on launch", async () => {
  ipc({ settings: { favourites: ["b"], favouritesOnly: true } });
  render(Grid, { store: createStationsStore(), player: createPlayerStore(), settings: createSettingsStore() });
  await waitFor(() => expect(names()).toEqual(["Station b"]));
  expect(screen.getByRole("button", { name: "Favourites" }).getAttribute("aria-pressed")).toBe("true");
  expect(screen.getByRole("button", { name: "All" }).getAttribute("aria-pressed")).toBe("false");
});

it("Favourites chip combines with a genre chip", async () => {
  ipc({ settings: { favourites: ["a", "b"], favouritesOnly: true } });
  render(Grid, { store: createStationsStore(), player: createPlayerStore(), settings: createSettingsStore() });
  await waitFor(() => expect(names()).toEqual(["Station a", "Station b"]));
  await fireEvent.click(screen.getByRole("button", { name: "Rock" }));
  expect(names()).toEqual(["Station b"]);
});

it("Player screen Favourite toggle reflects and toggles the loaded Station", async () => {
  const calls: Call[] = [];
  ipc({ player: ps({ stationId: "a" }), settings: { favourites: [], favouritesOnly: false }, calls });
  render(Player, { player: createPlayerStore(), stations: createStationsStore(), settings: createSettingsStore() });
  const btn = await screen.findByRole("button", { name: "Add to Favourites" });
  expect(btn.getAttribute("aria-pressed")).toBe("false");
  await fireEvent.click(btn);
  expect(calls).toContainEqual(["toggle_favourite", { stationId: "a" }]);
  await emit("settings-changed", { favourites: ["a"], favouritesOnly: false });
  const on = await screen.findByRole("button", { name: "Remove from Favourites" });
  expect(on.getAttribute("aria-pressed")).toBe("true");
});
