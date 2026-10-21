# Track history and Liked tracks spec (0.2.0)

Show a Station's recent tracks on the Player screen and let the user heart tracks into a Liked view. Terms (Station, Player screen, Now playing, Station history, Liked track) follow `GLOSSARY.md`. No ADR needed.

## Decisions (agreed, do not reopen)
- **Station history source**: `https://somafm.com/songs/<id>.json`, fetched by Rust. Fetched when the Player screen opens, and refetched when the Now playing title changes while the Player screen is visible. No timer, in-memory only, nothing when the window is hidden or the user is on the Grid.
- **Display**: a list under the Now playing track on the Player screen: time, "artist – title", and a heart per row; about 20 entries (whatever the endpoint returns). Fetch failure shows a quiet "History unavailable"; no retry loop.
- **Liked track**: the raw string is the source of truth, plus Station id and the timestamp liked. Artist/title are split on the first " - " best-effort for display and search links only (same split as `icy::now_playing`). History entries synthesize raw as "artist - title". Liking is allowed only for the current track and history entries.
- **Heart** is a toggle; dedupe on (Station id, raw).
- **Storage**: a separate `liked-tracks.json` in the app data dir via `tauri-plugin-store`, with its own `schemaVersion`; not in `settings.json`, so each file recovers from corruption independently (same lenient-load / `.bak` rules as `settings.rs`).
- **UI**: a new Liked view reachable from a Grid header button; hearts on the Player screen current track and in the NowPlayingBar. No tray entry in 0.2.0.
- **Liked view**: newest first. Rows show the track, Station name and date liked. Clicking the Station name plays it. Per row: copy text, "Search on" Bandcamp / Discogs / MusicBrainz (opens in the system browser), remove. No export, no sync.

Source: grilling session with the user, 2026-10-21.

## Tickets
01 history fetch (Rust) -> 02 history list UI -> 03 liked-tracks store (Rust) -> 04 hearts UI -> 05 Liked view -> 06 release 0.2.0. 03 may be done before 01/02; 04 needs 02 and 03.

Every ticket: test-first per the v1 test strategy (Rust reducer/unit tests, shell integration with stub servers + `NullSink`, Vitest + `mockIPC`); the gate is `pnpm check` (use `pnpm check:toolbox` on this host).

## Out of scope
- Export or sync of Liked tracks (export is a possible later idea).
- A tray entry for likes.
- Persisting Station history, or any polling timer.
- Liking tracks from anywhere other than the current track and history entries.
- Scrobbling or accounts.
