# 13 Spec layout and builder ticket breakdown

Type: grilling
Status: resolved
Blocked by: 09, 11, 12

## Question

What does the final build-ready spec look like, and how is the build split into implementation tickets? Decide: the spec's location and sections (pulling together every decision on this map), the vertical slices handed to builders, their order and blocking, and each slice's definition of done.

## Answer

Resolved with the user, 2026-10-16.

1. **Location**: new feature dir `.scratch/somafm-v1/` with `spec.md` and builder tickets `issues/01-…09-…`. Builder tickets carry `Status: ready-for-agent` and `Blocked by:` lines. The wayfinder map in `somafm-app/` stays as the record of how the decisions were made.
2. **Self-contained spec**: the spec restates every decision as a requirement, with a "Source: [ticket name]" link per section. A builder never needs to open the wayfinder tickets.
3. **Sections**: (1) Purpose & scope; (2) Stack & project layout; (3) Architecture, pinning the exact `PlayerState` type and command/event names; (4) Station feed & cache; (5) Playback: native audio, quality, ICY now-playing, reconnect/error state machine, timeouts; (6) UI; (7) Tray, media keys, overlay, close-to-tray, platform quirks; (8) Settings schema v1 & migration; (9) Testing, `check`, shared DoD, manual smoke checklist; (10) Build & install; (11) Out of scope.
4. **Slices** (blocked by):
   1. Scaffold + Open Props + `check` script (none)
   2. Station grid: feed fetch + cache + offline fallback, Tile wall (1)
   3. Play a station: `.pls` -> Mirror -> native decode, `PlayerState` reducer + snapshot events, player screen (stop, volume), back to grid with active ring + now-playing bar (2). Kept whole as the end-to-end tracer bullet.
   4. Now playing: ICY artist/title on the player and bar (3)
   5. Stream errors & reconnect: backoff, timeouts, error states, Retry UI (3)
   6. Settings & favourites: store + migrations, persisted volume/quality/lastStation, favourites + filter chip, window-state (3)
   7. Tray & close-to-tray (4, 6)
   8. Media keys & OS overlay (4)
   9. Local build & install + manual smoke run (5, 7, 8)
5. **Definition of done**: shared DoD in spec §9: `check` passes clean; core Rust test-first; new behaviour tested per the test strategy; the slice's smoke-checklist lines pass on Linux (Bazzite KDE). Each ticket adds 3–6 acceptance criteria and names the smoke lines it owns. macOS/Windows never block a slice.
6. **Next step**: write the spec and the nine builder tickets as the effort's final step (not a new map), then mark the map complete.
