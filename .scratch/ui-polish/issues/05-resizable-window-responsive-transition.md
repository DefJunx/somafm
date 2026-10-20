# 05 Resizable window, responsive layouts and view transition

Status: resolved
Blocked by: 04

## Goal
Make the window resizable (min 480x520) with Grid and Player screen layouts that respond to width, and add a Grid<->Player transition.

## Spec sections
- [Decisions](../spec.md#decisions-agreed-do-not-reopen) (Window, Interaction pass)
- [Audit findings](../spec.md#audit-findings) (no narrow breakpoint, no view transition)

## Acceptance criteria
- [ ] `src-tauri/tauri.conf.json`: `resizable: true`, `minWidth: 480`, `minHeight: 520`, default size stays 800x600. This is the only non-web file touched; `pnpm check:rust` still passes.
- [ ] Grid uses auto-fill/minmax columns so Tile count per row adapts from 480 px upward; chips wrap or scroll horizontally without clipping.
- [ ] Player screen two-column layout (`Player.svelte` ~81) collapses to one column below a width breakpoint (about 640 px); artwork scales, controls stay reachable, no horizontal scrollbar at 480 px.
- [ ] NowPlayingBar remains usable at 480 px (truncation, no overflow).
- [ ] `App.svelte` Grid<->Player switch uses a short fade/slide (motion tokens); with `prefers-reduced-motion: reduce` the swap is instant. Focus lands sensibly on the new view and Grid scroll position is preserved when returning.
- [ ] All existing tests green (`src/lib/*.test.ts`, `src/views/views.test.ts`); `pnpm check` (or `pnpm check:toolbox`) passes in full.

## Manual verification (user)
1. `pnpm tauri:dev`; drag the window edges: it resizes, cannot go below 480x520.
2. At 480, 640 and 1000 px wide: Grid columns adapt; Player screen goes single-column when narrow; nothing overflows horizontally.
3. Open a Station and go back: smooth fade/slide, scroll position kept; with OS "reduce motion" on, no animation.
4. Run `pnpm check:toolbox` and confirm it passes; then close out the UI-polish plan.
