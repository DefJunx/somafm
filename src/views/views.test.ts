import { afterEach, expect, it } from "vitest";
import { render, screen, cleanup, fireEvent, waitFor } from "@testing-library/svelte";
import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import Grid from "./Grid.svelte";
import Player from "./Player.svelte";
import { createStationsStore } from "../lib/stations.svelte";
import { station } from "../lib/fixtures";
import type { StationsSnapshot } from "../lib/api";

afterEach(() => { cleanup(); clearMocks(); });

const feed: StationsSnapshot = {
  loaded: true,
  stations: [station("a", ["Ambient"]), station("b", ["Rock"]), station("c", ["Ambient"])],
};

it("renders the placeholder Player", () => {
  render(Player);
  expect(screen.getByRole("heading", { name: "Player" })).toBeTruthy();
});

it("shows Tiles in feed order and filters by genre chip", async () => {
  mockIPC(() => feed, { shouldMockEvents: true });
  render(Grid, { store: createStationsStore() });
  await waitFor(() => expect(screen.getAllByRole("img")).toHaveLength(3));
  const ambient = screen.getByRole("button", { name: "Ambient" });
  await fireEvent.click(ambient);
  expect(ambient.getAttribute("aria-pressed")).toBe("true");
  expect(screen.getAllByRole("img").map((i) => i.getAttribute("alt"))).toEqual(["Station a", "Station c"]);
  await fireEvent.click(screen.getByRole("button", { name: "All" }));
  expect(screen.getAllByRole("img")).toHaveLength(3);
});

it("falls back to the station name when an image fails", async () => {
  mockIPC(() => feed, { shouldMockEvents: true });
  render(Grid, { store: createStationsStore() });
  const imgs = await waitFor(() => screen.getAllByRole("img"));
  await fireEvent.error(imgs[0]);
  expect(screen.getAllByRole("img")).toHaveLength(2);
  expect(screen.getByText("Station a")).toBeTruthy();
});

it("shows the empty state and Retry refreshes", async () => {
  mockIPC((cmd) => (cmd === "refresh_stations" ? feed : { stations: [], loaded: false }), { shouldMockEvents: true });
  render(Grid, { store: createStationsStore() });
  await screen.findByText("Can't reach SomaFM");
  await fireEvent.click(screen.getByRole("button", { name: "Retry" }));
  await waitFor(() => expect(screen.getAllByRole("img")).toHaveLength(3));
});

it("does not flash the empty state before the first snapshot", async () => {
  mockIPC(() => new Promise(() => {}), { shouldMockEvents: true });
  render(Grid, { store: createStationsStore() });
  await Promise.resolve();
  expect(screen.queryByText("Can't reach SomaFM")).toBeNull();
});

it("retries the image fallback when the image URL changes", async () => {
  const store = createStationsStore();
  mockIPC(() => feed, { shouldMockEvents: true });
  render(Grid, { store });
  const imgs = await waitFor(() => screen.getAllByRole("img"));
  await fireEvent.error(imgs[0]);
  expect(screen.getAllByRole("img")).toHaveLength(2);
  await emit("stations-updated", {
    loaded: true,
    stations: [station("a", ["Ambient"], { largeImage: "https://img.example/new.png" }), ...feed.stations.slice(1)],
  });
  await waitFor(() => expect(screen.getAllByRole("img")).toHaveLength(3));
});
