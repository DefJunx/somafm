import { afterEach, expect, it, vi } from "vitest";
import { render, screen, cleanup, fireEvent, waitFor, within } from "@testing-library/svelte";
import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import Liked from "./Liked.svelte";
import Grid from "./Grid.svelte";
import App from "../App.svelte";
import { createStationsStore } from "../lib/stations.svelte";
import { createPlayerStore, initialPlayerState } from "../lib/player.svelte";
import { createLikedStore } from "../lib/liked.svelte";
import { station } from "../lib/fixtures";
import type { LikedTrack } from "../lib/api";

afterEach(() => { cleanup(); clearMocks(); vi.unstubAllGlobals(); });

const feed = { loaded: true, stations: [station("a"), station("b")] };
type Call = [string, unknown];
const L = (stationId: string, raw: string, likedAt: number): LikedTrack => ({ stationId, raw, likedAt });
const local = (d: number, h = 12, m = 0) => new Date(2026, 9, d, h, m).getTime();
const NOW = local(21, 18);
const tracks = [L("b", "Y - Two - Mix", local(21, 9, 5)), L("gone", "Solo", local(20)), L("a", "X - One", local(19))];

function ipc(liked: LikedTrack[], calls: Call[] = []) {
  mockIPC((cmd, args) => {
    calls.push([cmd, args]);
    switch (cmd) {
      case "get_player_state": return initialPlayerState;
      case "get_stations": case "refresh_stations": return feed;
      case "get_settings": return { favourites: [], favouritesOnly: false };
      case "list_liked": return liked;
    }
  }, { shouldMockEvents: true });
}
function renderLiked(onback?: () => void) {
  render(Liked, { onback, now: () => NOW, player: createPlayerStore(), stations: createStationsStore(), liked: createLikedStore() });
}
const rows = () => screen.getAllByRole("listitem");
const ready = async (n = 3) => { await waitFor(() => expect(rows()).toHaveLength(n)); };

it("lists Liked tracks newest first, split into title and artist lines, grouped by day", async () => {
  ipc(tracks);
  renderLiked();
  await ready();
  const [r0, r1, r2] = rows();
  expect(within(r0).getByText("Two - Mix")).toBeTruthy();
  expect(within(r0).getByText("Y")).toBeTruthy();
  expect(r0.querySelector("time")?.getAttribute("datetime")).toBe(new Date(local(21, 9, 5)).toISOString());
  expect(r1.textContent).toContain("Solo");
  expect(r1.querySelector(".artist")).toBeNull(); // blank artist: title line only
  expect(within(r2).getByText("One")).toBeTruthy();
  expect(screen.getAllByRole("heading", { level: 2 }).map((h) => h.textContent)).toEqual(["Today", "Yesterday", "This week"]);
});

it("a Station missing from the feed shows its id and offers no play button", async () => {
  ipc(tracks);
  renderLiked();
  await ready();
  expect(within(rows()[1]).queryByRole("button", { name: /^Play/ })).toBeNull();
  expect(rows()[1].textContent).toContain("gone");
});

it("the thumbnail and the Station name both play that Station and stay on Liked", async () => {
  const calls: Call[] = [];
  ipc(tracks, calls);
  renderLiked();
  await ready();
  const plays = within(rows()[2]).getAllByRole("button", { name: "Play Station a" });
  expect(plays).toHaveLength(2);
  await fireEvent.click(plays[0]);
  await fireEvent.click(plays[1]);
  expect(calls.filter(([c]) => c === "play")).toEqual([["play", { stationId: "a" }], ["play", { stationId: "a" }]]);
  expect(screen.getByRole("heading", { name: "Liked" })).toBeTruthy();
});

it("copy writes the raw string and confirms with Copied, announced politely", async () => {
  const writeText = vi.fn().mockResolvedValue(undefined);
  vi.stubGlobal("navigator", { ...navigator, clipboard: { writeText } });
  ipc(tracks);
  renderLiked();
  await ready();
  await fireEvent.click(within(rows()[0]).getByRole("button", { name: "Copy Y – Two - Mix" }));
  expect(writeText).toHaveBeenCalledWith("Y - Two - Mix");
  await waitFor(() => expect(within(rows()[0]).getByRole("button", { name: "Copied Y – Two - Mix" })).toBeTruthy());
  expect(document.querySelector('[aria-live="polite"]:not([role])')?.textContent).toBe("Copied");
});

it("Search on opens each service in the system browser", async () => {
  const calls: Call[] = [];
  ipc(tracks, calls);
  renderLiked();
  await ready();
  const row = rows()[2];
  const group = within(row).getByRole("group", { name: "Search on" });
  await fireEvent.click(within(group).getByRole("button", { name: "Bandcamp" }));
  await fireEvent.click(within(group).getByRole("button", { name: "Discogs" }));
  await fireEvent.click(within(group).getByRole("button", { name: "MusicBrainz" }));
  const urls = calls.filter(([c]) => c === "plugin:opener|open_url").map(([, a]) => (a as { url: string }).url);
  expect(urls).toEqual([
    "https://bandcamp.com/search?q=X%20One",
    "https://www.discogs.com/search/?q=X%20One&type=all",
    "https://musicbrainz.org/search?query=X%20One&type=recording",
  ]);
});

it("remove hides the row at once, asks the backend, and offers Undo that restores the exact track", async () => {
  const calls: Call[] = [];
  ipc(tracks, calls);
  renderLiked();
  await ready();
  await fireEvent.click(within(rows()[2]).getByRole("button", { name: "Remove X – One" }));
  expect(calls).toContainEqual(["remove_liked", { stationId: "a", raw: "X - One" }]);
  await waitFor(() => expect(rows()).toHaveLength(2));
  const toast = screen.getByRole("status");
  expect(toast.textContent).toContain("Removed One");
  await fireEvent.click(within(toast).getByRole("button", { name: "Undo" }));
  expect(calls).toContainEqual(["restore_liked", { track: tracks[2] }]);
  await waitFor(() => expect(rows()).toHaveLength(3));
  expect(screen.queryByText(/^Removed/)).toBeNull();
});

it("the Undo toast goes away after a few seconds and a new removal replaces it", async () => {
  ipc(tracks);
  renderLiked();
  await ready();
  await fireEvent.click(within(rows()[2]).getByRole("button", { name: "Remove X – One" }));
  await screen.findByText("Removed One");
  await fireEvent.click(within(rows()[1]).getByRole("button", { name: "Remove Solo" }));
  await screen.findByText("Removed Solo");
  expect(screen.queryByText("Removed One")).toBeNull();
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
  try {
    await fireEvent.click(within(rows()[0]).getByRole("button", { name: "Remove Y – Two - Mix" }));
    await screen.findByText("Removed Two - Mix");
    await vi.advanceTimersByTimeAsync(5100);
    expect(screen.queryByText(/^Removed/)).toBeNull();
  } finally { vi.useRealTimers(); }
});

it("filters as you type on track and Station name, hides empty groups, and clears with Escape", async () => {
  ipc(tracks);
  renderLiked();
  await ready();
  const input = screen.getByRole("searchbox", { name: "Filter liked tracks" });
  await fireEvent.input(input, { target: { value: "STATION B" } });
  await waitFor(() => expect(rows()).toHaveLength(1));
  expect(screen.getAllByRole("heading", { level: 2 }).map((h) => h.textContent)).toEqual(["Today"]);
  await fireEvent.input(input, { target: { value: "zzz" } });
  await screen.findByText("No liked tracks match “zzz”");
  await fireEvent.click(screen.getByRole("button", { name: "Clear filter" }));
  await ready();
  await fireEvent.input(input, { target: { value: "one" } });
  await waitFor(() => expect(rows()).toHaveLength(1));
  await fireEvent.keyDown(input, { key: "Escape" });
  await ready();
  expect((input as HTMLInputElement).value).toBe("");
});

it("shows an empty state with Browse stations, and Back", async () => {
  const onback = vi.fn();
  ipc([]);
  renderLiked(onback);
  await screen.findByText("No liked tracks yet");
  expect(screen.getByText("Tap the heart next to a playing track to save it here.")).toBeTruthy();
  expect(screen.queryByRole("listitem")).toBeNull();
  await fireEvent.click(screen.getByRole("button", { name: "Browse stations" }));
  await fireEvent.click(screen.getByRole("button", { name: "Back" }));
  expect(onback).toHaveBeenCalledTimes(2);
});

it("Grid header button opens the Liked view via App, Back returns to the Grid", async () => {
  ipc(tracks);
  render(App);
  await fireEvent.click(await screen.findByRole("button", { name: "Liked" }));
  await screen.findByRole("heading", { name: "Liked" });
  expect(document.getElementById("grid-view")?.hidden).toBe(true);
  await fireEvent.click(screen.getByRole("button", { name: "Back" }));
  await waitFor(() => expect(document.getElementById("grid-view")?.hidden).toBe(false));
  expect(screen.queryByRole("heading", { name: "Liked" })).toBeNull();
});

it("Grid shows the Liked button and calls onopenliked", async () => {
  ipc([]);
  const onopenliked = vi.fn();
  render(Grid, { store: createStationsStore(), player: createPlayerStore(), liked: createLikedStore(), onopenliked });
  await fireEvent.click(await screen.findByRole("button", { name: "Liked" }));
  expect(onopenliked).toHaveBeenCalled();
});
