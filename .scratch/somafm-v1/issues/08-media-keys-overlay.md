# 08 Media keys & OS overlay

Status: resolved
Blocked by: 04

## Goal
Wire hardware media keys and the OS now-playing overlay to the player via souvlaki.

## Spec sections
- [§7 Tray, media keys](../spec.md#7-tray-media-keys-os-overlay-close-to-tray) (media keys / overlay table, per-OS)
- [§5 Playback](../spec.md#5-playback) (5.3 artist/title split)

## Acceptance criteria
- [ ] `media_controls` (souvlaki: MPRIS on Linux, SMTC on Windows, macOS now-playing) maps Play/Pause to stop / live reconnect, Stop to `stop`; with no loaded Station Play starts `lastStationId`, and does nothing if none; Next/Previous, seek and OS volume are not advertised/ignored.
- [ ] Overlay reports Playing while `playing`/`connecting`, and Paused (never Stopped) when `stopped` with a loaded Station and in `error`; updates on every `player-state`.
- [ ] Metadata: artist = ICY artist (station name if none), title = ICY title, album = Station name, artwork = Station image URL; no duration/position. The mapping is a pure function with table tests.
- [ ] Windows binding uses the main window HWND (window hidden, never destroyed).

## Smoke checklist
S14, S15. Shared Definition of Done in [§9 Testing & DoD](../spec.md#9-testing--the-check-command) applies.

## Notes
- Slice 06 may not be merged yet: read `lastStationId` through the `settings` interface if present, otherwise from `PlayerState.stationId`.
- Verify on Linux via the KDE media widget / `playerctl`.
- User verified S14-S15 manually on 2026-10-18 (media keys and KDE media widget). Resolved.
