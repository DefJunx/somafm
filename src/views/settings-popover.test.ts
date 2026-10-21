import { afterEach, expect, it } from "vitest";
import { render, screen, cleanup, fireEvent, waitFor } from "@testing-library/svelte";
import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import Grid from "./Grid.svelte";
import { createStationsStore } from "../lib/stations.svelte";
import { createPlayerStore, initialPlayerState } from "../lib/player.svelte";
import { createSettingsStore } from "../lib/settings.svelte";
import { station } from "../lib/fixtures";
import type { SettingsView } from "../lib/api";

afterEach(() => { cleanup(); clearMocks(); });

type Call = [string, unknown];

function setup(settings: Partial<SettingsView> = {}) {
  const calls: Call[] = [];
  mockIPC(
    (cmd, args) => {
      if (cmd === "get_settings") return { favourites: [], favouritesOnly: false, fadeAudio: true, showVisualizer: true, ...settings };
      if (cmd === "get_player_state") return initialPlayerState;
      if (cmd === "get_stations" || cmd === "refresh_stations") return { loaded: true, stations: [station("a")] };
      if (cmd === "list_liked") return [];
      calls.push([cmd, args]);
    },
    { shouldMockEvents: true },
  );
  render(Grid, { store: createStationsStore(), player: createPlayerStore(), settings: createSettingsStore() });
  return calls;
}

const gear = () => screen.getByRole("button", { name: "Settings" });

it("the gear opens a popover with the two labelled toggles and Escape closes it, returning focus to the gear", async () => {
  setup();
  expect(screen.queryByRole("dialog")).toBeNull();
  expect(gear().getAttribute("aria-expanded")).toBe("false");
  await fireEvent.click(gear());
  expect(screen.getByRole("dialog", { name: "Settings" })).toBeTruthy();
  expect(gear().getAttribute("aria-expanded")).toBe("true");
  expect(screen.getByRole("checkbox", { name: "Fade audio" })).toBeTruthy();
  expect(screen.getByRole("checkbox", { name: "Show visualizer" })).toBeTruthy();
  // keyboard: focus moves into the popover on open
  await waitFor(() => expect(document.activeElement).toBe(screen.getByRole("checkbox", { name: "Fade audio" })));
  await fireEvent.keyDown(document.activeElement!, { key: "Escape" });
  expect(screen.queryByRole("dialog")).toBeNull();
  expect(document.activeElement).toBe(gear());
});

it("the gear toggles the popover and a click outside closes it", async () => {
  setup();
  await fireEvent.click(gear());
  await fireEvent.click(gear());
  expect(screen.queryByRole("dialog")).toBeNull();
  await fireEvent.click(gear());
  await fireEvent.pointerDown(document.body);
  expect(screen.queryByRole("dialog")).toBeNull();
});

it("toggling Fade audio and Show visualizer invokes their commands and follows settings-changed", async () => {
  const calls = setup({ fadeAudio: true, showVisualizer: false });
  await fireEvent.click(gear());
  const fade = screen.getByRole<HTMLInputElement>("checkbox", { name: "Fade audio" });
  const viz = screen.getByRole<HTMLInputElement>("checkbox", { name: "Show visualizer" });
  await waitFor(() => expect(viz.checked).toBe(false));
  expect(fade.checked).toBe(true);
  await fireEvent.click(fade);
  expect(calls).toContainEqual(["set_fade_audio", { value: false }]);
  await fireEvent.click(viz);
  expect(calls).toContainEqual(["set_show_visualizer", { value: true }]);
  // Rust confirms; the checkboxes follow the event.
  await emit("settings-changed", { favourites: [], favouritesOnly: false, fadeAudio: false, showVisualizer: true });
  await waitFor(() => expect(fade.checked).toBe(false));
  expect(viz.checked).toBe(true);
});
