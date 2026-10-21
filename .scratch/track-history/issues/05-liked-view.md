# 05 Liked view

Status: resolved
Blocked by: 03

## Goal
A Liked view, reachable from the Grid header, lists Liked tracks with actions.

## Spec sections
- [Decisions](../spec.md#decisions-agreed-do-not-reopen) (Liked view, UI)

## Acceptance criteria
- [ ] A Grid header button opens a Liked view (`src/views/Liked.svelte`, toggled from `App.svelte` like Grid/Player), with a back control; empty state when nothing is liked.
- [ ] Rows are newest first and show the track (artist – title split on the first " - ", or the raw string when it has no split), the Station name from the feed, and the date liked.
- [ ] Clicking the Station name plays that Station; a Liked track whose Station is missing from the feed shows the id and is not clickable.
- [ ] Per row: copy text (writes the raw string to the clipboard), "Search on" Bandcamp / Discogs / MusicBrainz opening a URL-encoded search in the system browser (via the existing opener mechanism; check capabilities), and remove.
- [ ] A pure helper builds the search URLs and splits artist/title; unit tested including strings with extra " - " and none.
- [ ] Vitest with `mockIPC` covers list order, play-station click, copy, remove and the empty state.
- [ ] `pnpm check` passes.

## Manual verification (user)
1. Like a few tracks, open Liked: newest first, correct Station names.
2. Click a Station name: it starts playing.
3. Use copy, each Search link, and remove.

## Comments
Frontend done (Liked view, helpers, tests; Search links call `openUrl` in `src/lib/api.ts` -> `plugin:opener|open_url`). Left open: the app has no opener plugin, so Search on does nothing in the real app until Rust adds `tauri-plugin-opener` (Cargo.toml, `.plugin(tauri_plugin_opener::init())` in lib.rs, `opener:allow-open-url` in `src-tauri/capabilities/default.json`, plus `@tauri-apps/plugin-opener` is NOT needed since api.ts invokes the command directly). Resolved: plugin added with `opener:allow-open-url` scoped to the three search URLs. Manual smoke run not done.
