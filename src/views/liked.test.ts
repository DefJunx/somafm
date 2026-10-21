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
const tracks = [L("b", "Y - Two - Mix", Date.UTC(2026, 9, 21, 12)), L("gone", "Solo", Date.UTC(2026, 9, 20, 12)), L("a", "X - One", Date.UTC(2026, 9, 19, 12))];

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
  render(Liked, { onback, player: createPlayerStore(), stations: createStationsStore(), liked: createLikedStore() });
}
const rows = () => screen.getAllByRole("listitem");

it("lists Liked tracks in the given (newest first) order with station name and date", async () => {
  ipc(tracks);
  renderLiked();
  await waitFor(() => expect(rows()).toHaveLength(3));
  const [r0, r1, r2] = rows();
  expect(r0.textContent).toContain("Y – Two - Mix");
  expect(within(r0).getByRole("button", { name: "Station b" })).toBeTruthy();
  expect(r0.querySelector("time")?.getAttribute("datetime")).toBe("2026-10-21T12:00:00.000Z");
  expect(r1.textContent).toContain("Solo"); // no split: raw string
  expect(r2.textContent).toContain("X – One");
});

it("a Station missing from the feed shows its id and is not clickable", async () => {
  ipc(tracks);
  renderLiked();
  await waitFor(() => expect(rows()).toHaveLength(3));
  expect(within(rows()[1]).queryByRole("button", { name: "gone" })).toBeNull();
  expect(rows()[1].textContent).toContain("gone");
});

it("clicking the Station name plays that Station", async () => {
  const calls: Call[] = [];
  ipc(tracks, calls);
  renderLiked();
  await waitFor(() => expect(rows()).toHaveLength(3));
  await fireEvent.click(within(rows()[2]).getByRole("button", { name: "Station a" }));
  expect(calls).toContainEqual(["play", { stationId: "a" }]);
});

it("copy writes the raw string to the clipboard", async () => {
  const writeText = vi.fn().mockResolvedValue(undefined);
  vi.stubGlobal("navigator", { ...navigator, clipboard: { writeText } });
  ipc(tracks);
  renderLiked();
  await waitFor(() => expect(rows()).toHaveLength(3));
  await fireEvent.click(within(rows()[0]).getByRole("button", { name: "Copy text" }));
  expect(writeText).toHaveBeenCalledWith("Y - Two - Mix");
});

it("Search on opens each service in the system browser", async () => {
  const calls: Call[] = [];
  ipc(tracks, calls);
  renderLiked();
  await waitFor(() => expect(rows()).toHaveLength(3));
  const row = rows()[2];
  await fireEvent.click(within(row).getByRole("button", { name: "Bandcamp" }));
  await fireEvent.click(within(row).getByRole("button", { name: "Discogs" }));
  await fireEvent.click(within(row).getByRole("button", { name: "MusicBrainz" }));
  const urls = calls.filter(([c]) => c === "plugin:opener|open_url").map(([, a]) => (a as { url: string }).url);
  expect(urls).toEqual([
    "https://bandcamp.com/search?q=X%20One",
    "https://www.discogs.com/search/?q=X%20One&type=all",
    "https://musicbrainz.org/search?query=X%20One&type=recording",
  ]);
});

it("remove asks the backend and the row goes when liked-changed arrives", async () => {
  const calls: Call[] = [];
  ipc(tracks, calls);
  renderLiked();
  await waitFor(() => expect(rows()).toHaveLength(3));
  await fireEvent.click(within(rows()[2]).getByRole("button", { name: "Remove" }));
  expect(calls).toContainEqual(["remove_liked", { stationId: "a", raw: "X - One" }]);
  await emit("liked-changed", tracks.slice(0, 2));
  await waitFor(() => expect(rows()).toHaveLength(2));
});

it("shows an empty state and Back", async () => {
  const onback = vi.fn();
  ipc([]);
  renderLiked(onback);
  await screen.findByText("No liked tracks yet");
  expect(screen.queryByRole("listitem")).toBeNull();
  await fireEvent.click(screen.getByRole("button", { name: "Back" }));
  expect(onback).toHaveBeenCalled();
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
