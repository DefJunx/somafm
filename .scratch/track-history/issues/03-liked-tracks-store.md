# 03 Liked tracks store and commands (Rust)

Status: resolved
Blocked by:

## Goal
Liked tracks persist in their own `liked-tracks.json`, with commands and an event for the frontend.

## Spec sections
- [Decisions](../spec.md#decisions-agreed-do-not-reopen) (Liked track, Heart, Storage)

## Acceptance criteria
- [x] A `liked` module wraps `tauri-plugin-store` on `liked-tracks.json` with `schemaVersion` 1 and an empty MIGRATIONS table; lenient load, corrupt file moved to `liked-tracks.json.bak`, newer `schemaVersion` tolerated; mirrors `settings.rs` behaviour. Test-first with temp files.
- [x] A liked entry is `{ raw, stationId, likedAt }`; camelCase keys. Artist/title are not stored.
- [x] `toggle_like(station_id, raw)` adds or removes, deduping on (Station id, raw); `list_liked` returns newest first; `remove_liked(station_id, raw)`; a `liked-changed` event is emitted after each change. Pure toggle/dedupe logic is unit-tested.
- [x] Corrupt `liked-tracks.json` does not affect `settings.json` and vice versa (test).
- [x] `pnpm check` passes.

## Notes
- Rust is the only writer. Station name is looked up at display time from the feed; the file keeps only the id.
