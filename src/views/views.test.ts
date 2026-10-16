import { expect, it } from "vitest";
import { render, screen } from "@testing-library/svelte";
import Grid from "./Grid.svelte";
import Player from "./Player.svelte";

it("renders the placeholder views", () => {
  const g = render(Grid);
  expect(screen.getByRole("heading", { name: "Grid" })).toBeTruthy();
  g.unmount();
  render(Player);
  expect(screen.getByRole("heading", { name: "Player" })).toBeTruthy();
});
