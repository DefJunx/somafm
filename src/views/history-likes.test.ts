import { afterEach, expect, it } from "vitest";
import { render, screen, cleanup, fireEvent, waitFor, within } from "@testing-library/svelte";
import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import Grid from "./Grid.svelte";
import Player from "./Player.svelte";
import { createStationsStore } from "../lib/stations.svelte";
import { createPlayerStore, initialPlayerState } from "../lib/player.svelte";
import { createSettingsStore } from "../lib/settings.svelte";
import { createHistoryStore } from "../lib/history.svelte";
import { createLikedStore } from "../lib/liked.svelte";
import { station } from "../lib/fixtures";
import type { HistoryEntry, LikedTrack, PlayerState } from "../lib/api";

afterEach(() => { cleanup(); clearMocks(); });

const feed = { loaded: true, stations: [station("a"), station("b")] };
const ps = (over: Partial<PlayerState> = {}): PlayerState => ({ ...initialPlayerState, ...over });
const np = (artist: string, title: string) => ({ raw: `${artist} - ${title}`, artist, title });
const playing = (stationId: string, nowPlaying: PlayerState["nowPlaying"] = null) => ps({ stationId, status: { kind: "playing" }, nowPlaying });
const entry = (artist: string, title: string): HistoryEntry => ({ time: 1_700_000_000, artist, title, raw: artist ? `${artist} - ${title}` : title });
type Call = [string, unknown];

interface Opts {
  player: PlayerState;
  history?: (stationId: string) => HistoryEntry[] | Promise<HistoryEntry[]>;
  liked?: LikedTrack[];
  calls?: Call[];
}
function ipc(o: Opts) {
  mockIPC((cmd, args) => {
    o.calls?.push([cmd, args]);
    switch (cmd) {
      case "get_player_state": return o.player;
      case "get_stations": case "refresh_stations": return feed;
      case "get_settings": return { favourites: [], favouritesOnly: false };
      case "list_liked": return o.liked ?? [];
      case "toggle_like": return true;
      case "get_station_history": return o.history ? o.history((args as { stationId: string }).stationId) : [];
    }
  }, { shouldMockEvents: true });
}
const historyCalls = (calls: Call[]) => calls.filter(([c]) => c === "get_station_history");
const reducedMotion = () => {
  const original = window.matchMedia;
  Object.defineProperty(window, "matchMedia", { configurable: true, value: () => ({ matches: true } as MediaQueryList) });
  return () => Object.defineProperty(window, "matchMedia", { configurable: true, value: original });
};

function renderPlayer() {
  const stores = { player: createPlayerStore(), stations: createStationsStore(), settings: createSettingsStore(), history: createHistoryStore(), liked: createLikedStore() };
  render(Player, stores);
  return stores;
}
const settle = () => new Promise((r) => setTimeout(r, 20));

it("Player fetches the history on open and lists time and 'artist – title'", async () => {
  const calls: Call[] = [];
  ipc({ player: playing("a"), calls, history: () => [entry("X", "One"), entry("Y", "Two")] });
  renderPlayer();
  const list = await screen.findByRole("list", { name: "Station history" });
  const rows = within(list).getAllByRole("listitem");
  expect(rows).toHaveLength(2);
  expect(rows[0].textContent).toMatch(/\d{1,2}:\d{2}/);
  expect(rows[0].textContent).toContain("X – One");
  expect(rows[1].textContent).toContain("Y – Two");
  expect(historyCalls(calls)).toEqual([["get_station_history", { stationId: "a" }]]);
});

it("Player refetches when the Now playing title changes, and not on other changes", async () => {
  const calls: Call[] = [];
  ipc({ player: playing("a", np("X", "One")), calls, history: () => [entry("X", "One")] });
  renderPlayer();
  await screen.findByRole("list", { name: "Station history" });
  await emit("player-state", { ...playing("a", np("X", "One")), volume: 0.3 });
  await settle();
  expect(historyCalls(calls)).toHaveLength(1);
  await emit("player-state", playing("a", np("Y", "Two")));
  await waitFor(() => expect(historyCalls(calls)).toHaveLength(2));
});

it("Grid does not fetch the history", async () => {
  const calls: Call[] = [];
  ipc({ player: playing("a", np("X", "One")), calls });
  render(Grid, { store: createStationsStore(), player: createPlayerStore(), settings: createSettingsStore(), liked: createLikedStore() });
  await screen.findByRole("button", { name: "Play Station a" });
  await emit("player-state", playing("a", np("Y", "Two")));
  await settle();
  expect(historyCalls(calls)).toHaveLength(0);
});

it("a failed fetch shows 'History unavailable' without retrying; a later title change tries again", async () => {
  const calls: Call[] = [];
  let fail = true;
  ipc({ player: playing("a", np("X", "One")), calls, history: () => { if (fail) throw new Error("offline"); return [entry("X", "One")]; } });
  renderPlayer();
  await screen.findByText("History unavailable");
  await settle();
  expect(historyCalls(calls)).toHaveLength(1);
  fail = false;
  await emit("player-state", playing("a", np("Y", "Two")));
  await screen.findByRole("list", { name: "Station history" });
  expect(screen.queryByText("History unavailable")).toBeNull();
});

it("switching Station discards the old list and ignores a stale response", async () => {
  const pending: Record<string, (v: HistoryEntry[]) => void> = {};
  ipc({ player: playing("a"), history: (id) => new Promise((r) => (pending[id] = r)) });
  renderPlayer();
  await waitFor(() => expect(pending.a).toBeTypeOf("function"));
  await emit("player-state", playing("b"));
  await waitFor(() => expect(pending.b).toBeTypeOf("function"));
  pending.b([entry("B", "bt")]);
  await screen.findByText(/B – bt/);
  pending.a([entry("A", "at")]);
  await settle();
  expect(screen.queryByText(/A – at/)).toBeNull();
  expect(screen.getByText(/B – bt/)).toBeTruthy();
});

// --- hearts ---

it("heart on the current track toggles via toggle_like and follows liked-changed", async () => {
  const calls: Call[] = [];
  ipc({ player: playing("a", np("X", "One")), calls });
  renderPlayer();
  const heart = await screen.findByRole("button", { name: "Like X – One" });
  expect(heart.getAttribute("aria-pressed")).toBe("false");
  await fireEvent.click(heart);
  expect(calls).toContainEqual(["toggle_like", { stationId: "a", raw: "X - One" }]);
  await emit("liked-changed", [{ stationId: "a", raw: "X - One", likedAt: 1 }]);
  const liked = await screen.findByRole("button", { name: "Unlike X – One" });
  expect(liked.getAttribute("aria-pressed")).toBe("true");
  await emit("liked-changed", []);
  await screen.findByRole("button", { name: "Like X – One" });
});

it("history rows have hearts, and a row and the current track share one Liked entry when raw matches", async () => {
  const calls: Call[] = [];
  ipc({ player: playing("a", np("X", "One")), calls, history: () => [entry("X", "One"), entry("Y", "Two")], liked: [{ stationId: "a", raw: "X - One", likedAt: 1 }] });
  renderPlayer();
  const list = await screen.findByRole("list", { name: "Station history" });
  expect(within(list).getByRole("button", { name: "Unlike X – One" }).getAttribute("aria-pressed")).toBe("true");
  const other = within(list).getByRole("button", { name: "Like Y – Two" });
  expect(other.getAttribute("aria-pressed")).toBe("false");
  // current track heart is filled too
  expect(screen.getAllByRole("button", { pressed: true, name: /^Unlike X/ })).toHaveLength(2);
  await fireEvent.click(other);
  expect(calls).toContainEqual(["toggle_like", { stationId: "a", raw: "Y - Two" }]);
});

it("no heart on the current track when nothing is playing a track", async () => {
  ipc({ player: playing("a") });
  renderPlayer();
  await screen.findByRole("heading", { name: "Station a" });
  expect(screen.queryByRole("button", { name: /^(Like|Unlike) / })).toBeNull();
});

it("NowPlayingBar heart likes the current track and is absent without one", async () => {
  const calls: Call[] = [];
  ipc({ player: playing("a"), calls });
  render(Grid, { store: createStationsStore(), player: createPlayerStore(), settings: createSettingsStore(), liked: createLikedStore() });
  const bar = await screen.findByRole("complementary", { name: "Now playing" });
  expect(within(bar).queryByRole("button", { name: /^(Like|Unlike) / })).toBeNull();
  await emit("player-state", playing("a", np("X", "One")));
  await fireEvent.click(await within(bar).findByRole("button", { name: "Like X – One" }));
  expect(calls).toContainEqual(["toggle_like", { stationId: "a", raw: "X - One" }]);
  await emit("liked-changed", [{ stationId: "a", raw: "X - One", likedAt: 1 }]);
  await within(bar).findByRole("button", { name: "Unlike X – One" });
});

it("a history row with a blank artist shows just the title and likes the bare raw", async () => {
  const calls: Call[] = [];
  ipc({ player: playing("a"), calls, history: () => [entry("", "Bare")] });
  renderPlayer();
  const list = await screen.findByRole("list", { name: "Station history" });
  expect(within(list).getAllByRole("listitem")[0].textContent).toContain("Bare");
  expect(within(list).getAllByRole("listitem")[0].textContent).not.toContain("–");
  await fireEvent.click(within(list).getByRole("button", { name: "Like Bare" }));
  expect(calls).toContainEqual(["toggle_like", { stationId: "a", raw: "Bare" }]);
});

it("does not refetch when Now playing clears (stop or reconnect), only for a new track", async () => {
  const calls: Call[] = [];
  ipc({ player: playing("a", np("X", "One")), calls, history: () => [entry("X", "One")] });
  renderPlayer();
  await screen.findByRole("list", { name: "Station history" });
  await emit("player-state", playing("a", null));
  await settle();
  expect(historyCalls(calls)).toHaveLength(1);
  await emit("player-state", playing("a", np("Y", "Two")));
  await waitFor(() => expect(historyCalls(calls)).toHaveLength(2));
});

it("a track change while the window is hidden is fetched once on becoming visible", async () => {
  const calls: Call[] = [];
  ipc({ player: playing("a", np("X", "One")), calls, history: () => [entry("X", "One")] });
  renderPlayer();
  await screen.findByRole("list", { name: "Station history" });
  const hidden = (v: boolean) => {
    Object.defineProperty(document, "hidden", { value: v, configurable: true });
    document.dispatchEvent(new Event("visibilitychange"));
  };
  try {
    hidden(true);
    await emit("player-state", playing("a", np("Y", "Two")));
    await settle();
    expect(historyCalls(calls)).toHaveLength(1);
    hidden(false);
    await waitFor(() => expect(historyCalls(calls)).toHaveLength(2));
    await settle();
    expect(historyCalls(calls)).toHaveLength(2);
  } finally {
    Object.defineProperty(document, "hidden", { value: false, configurable: true });
  }
});

it("refetches after Player remount and keeps a request completed in the unmounted gap", async () => {
  const calls: Call[] = [];
  const pending: ((entries: HistoryEntry[]) => void)[] = [];
  ipc({ player: playing("a", np("X", "One")), calls, history: () => new Promise((resolve) => pending.push(resolve)) });
  const stores = { player: createPlayerStore(), stations: createStationsStore(), settings: createSettingsStore(), history: createHistoryStore(), liked: createLikedStore() };
  const restoreMatchMedia = reducedMotion();

  try {
    const first = render(Player, stores);
    await waitFor(() => expect(historyCalls(calls)).toHaveLength(1));
    first.unmount();
    pending[0]([entry("X", "One")]);
    await waitFor(() => expect(stores.history.entries).toEqual([entry("X", "One")]));

    const second = render(Player, stores);
    await waitFor(() => expect(historyCalls(calls)).toHaveLength(2));
    expect(stores.history.entries).toEqual([entry("X", "One")]);
    second.unmount();
  } finally {
    cleanup();
    restoreMatchMedia();
  }
});

it("ignores the prior mount's response after the remounted Player starts a request", async () => {
  const calls: Call[] = [];
  const pending: ((entries: HistoryEntry[]) => void)[] = [];
  ipc({ player: playing("a", np("X", "One")), calls, history: () => new Promise((resolve) => pending.push(resolve)) });
  const stores = { player: createPlayerStore(), stations: createStationsStore(), settings: createSettingsStore(), history: createHistoryStore(), liked: createLikedStore() };
  const restoreMatchMedia = reducedMotion();

  try {
    const first = render(Player, stores);
    await waitFor(() => expect(historyCalls(calls)).toHaveLength(1));
    first.unmount();

    const second = render(Player, stores);
    await waitFor(() => expect(historyCalls(calls)).toHaveLength(2));
    pending[0]([entry("old", "stale")]);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(stores.history.entries).toEqual([]);

    pending[1]([entry("new", "current")]);
    await waitFor(() => expect(stores.history.entries).toEqual([entry("new", "current")]));
    second.unmount();
  } finally {
    cleanup();
    restoreMatchMedia();
  }
});
