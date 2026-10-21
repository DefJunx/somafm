# 04 Hearts: current track, history rows, NowPlayingBar

Status: resolved
Blocked by: 02, 03

## Goal
The user can like the current track and history entries with a heart toggle.

## Spec sections
- [Decisions](../spec.md#decisions-agreed-do-not-reopen) (Liked track, Heart, UI)

## Acceptance criteria
- [x] A frontend liked store (`src/lib/liked.svelte.ts`, like `settings.svelte.ts`) loads `list_liked` and follows `liked-changed`.
- [x] A heart button (added to the inline SVG icon set, with filled/outline states and an accessible name) appears on the current track in the Player screen, on each history row, and in the NowPlayingBar. Filled when (Station id, raw) is liked.
- [x] Current-track raw is `NowPlaying.raw`; history rows synthesize raw as "artist - title". No heart when there is no current track.
- [x] Clicking toggles via `toggle_like`; no other track can be liked.
- [x] Vitest with `mockIPC`: heart toggles, state follows `liked-changed`, history row and current track dedupe to the same entry when raw matches, no heart without a track.
- [x] `pnpm check` passes.
