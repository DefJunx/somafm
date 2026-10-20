# 06 Settings & favourites: store + migrations, persisted prefs, favourites filter chip, window-state

Status: resolved
Blocked by: 03

## Goal
Persist volume, last Station, Favourites and chip state in a migrating store, add the Favourites UI, and restore window geometry.

## Spec sections
- [§8 Settings schema](../spec.md#8-settings-schema-v1--migration)
- [§3 Architecture](../spec.md#3-architecture-rust-vs-frontend) (`get_settings`, `toggle_favourite`, `set_favourites_only`, `settings-changed`)
- [§6 UI](../spec.md#6-ui) (Favourites chip, Favourite toggle, launch behaviour)

## Acceptance criteria
- [ ] `settings` wraps `tauri-plugin-store` with the v1 schema and defaults; ordered migrations, per-key lenient fallback, corrupt file -> `settings.json.bak`, newer `schemaVersion` tolerated; all test-first with temp files.
- [ ] Volume (debounced ~500 ms) and `lastStationId` persist; on launch the last Station loads as `stopped` (bottom bar shows Play), no autoplay; a missing last Station is treated as null without rewriting.
- [ ] `toggle_favourite`, `set_favourites_only`, `get_settings` and `settings-changed` work; Favourite toggle on the Player screen and the Favourites chip filter the Grid (feed order, missing ids hidden, never pruned); Vitest covers the chip.
- [ ] `tauri-plugin-window-state` restores window size/position.

## Smoke checklist
S9, S10, S11. Shared Definition of Done in [§9 Testing & DoD](../spec.md#9-testing--the-check-command) applies.

## Notes
- Rust `settings` is the only writer. Window state is a separate file by design.
- Tray (slice 07) reads favourites through this module and listens for `settings-changed`.
