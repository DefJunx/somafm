# 09 Settings schema and migration

Type: grilling
Status: resolved
Blocked by: 

## Question

What exactly does the tauri-plugin-store file hold? Decide: the key names, types and defaults (favourites, volume, quality, last station, any others), how a schema version is recorded and migrated, and what happens when a stored Favourite or last station no longer exists in the feed (hide, keep and grey out, or prune).

## Answer

Resolved with the user on 2026-10-16.

- **Shape** (one `tauri-plugin-store` file; Rust `settings` is the only writer; camelCase keys):

  ```json
  {
    "schemaVersion": 1,
    "favourites": [],
    "volume": 0.8,
    "quality": "mp3",
    "lastStationId": null,
    "favouritesOnly": false
  }
  ```

  - `favourites`: SomaFM feed station `id`s, a set (deduplicated). Displayed in feed order; no manual reordering.
  - `volume`: float 0.0-1.0, default 0.8, clamped on load.
  - `quality`: `"mp3" | "aac128"`, default `"mp3"`.
  - `lastStationId`: string or null, default null. Written when `play` successfully starts a station, not on stop.
  - `favouritesOnly`: persisted state of the Favourites filter chip, default false.
- **Launch**: the last station is restored into `PlayerState` as `stopped`. It shows in the bottom bar with Play, and nothing autoplays.
- **Missing from feed**: a Favourite whose id isn't in the current feed stays in the store and is hidden from the grid. It is never auto-pruned, because the cache may be stale or the station may be gone only temporarily. If `lastStationId` is missing, it is treated as null at load and isn't rewritten until the next change.
- **Versioning**: `schemaVersion` integer; ordered `vN -> vN+1` migrations run in Rust on load, then save.
  - No file: use the defaults.
  - File that can't be parsed: rename it to `settings.json.bak`, use the defaults, and log it.
  - Invalid value for one key: use that key's default and keep the rest.
  - `schemaVersion` newer than the app knows: read the known keys and leave the version number alone.
- **Writes**: save after every change that should persist; volume saves are debounced (~500 ms).
- **Window geometry**: saved by `tauri-plugin-window-state` in its own file, outside this schema.

Note (2026-10-16): the sleep timer was later ruled out of scope; references to it were removed above.
