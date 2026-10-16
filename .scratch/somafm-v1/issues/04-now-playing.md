# 04 Now playing: ICY artist/title on the player and bar

Status: resolved
Blocked by: 03

## Goal
Parse ICY `StreamTitle` inline and show artist and title on the Player screen and the bottom bar.

## Spec sections
- [§5 Playback](../spec.md#5-playback) (5.1 ICY adapter, 5.3)
- [§3 Architecture](../spec.md#3-architecture-rust-vs-frontend) (`NowPlaying`, `player-state`)
- [§6 UI](../spec.md#6-ui) (Player screen, bottom bar)

## Acceptance criteria
- [x] ICY framing in the stream adapter (`Icy-MetaData: 1`, `icy-metaint`) strips metadata from audio bytes; tested against a stub server emitting ICY-framed tone fixtures.
- [x] `StreamTitle` parser table-tested (test-first): no separator, several separators, empty, non-UTF-8, stray quotes; split on the first " - " into artist/title; no separator -> `artist: None`.
- [x] `nowPlaying` updates via `player-state`; kept across reconnects, cleared on `stop` and Station change.
- [x] Player screen and bottom bar show `artist - title` (station name as artist when none); Vitest covers both shapes and the cleared state.

## Smoke checklist
S6. Shared Definition of Done in [§9 Testing & DoD](../spec.md#9-testing--the-check-command) applies.

## Notes
- Do not use feed `lastPlaying` for the Active station.

## Comments
- Implemented in `src-tauri/src/player/icy.rs` (parser, split, `IcyReader`); `shell.rs` requests `Icy-MetaData: 1`, reads `icy-metaint`, wraps the network reader in `IcyReader` before `StreamSource`/symphonia. Titles reach the reducer as `Event::TitleChanged{attempt, raw}`.
- Smoke S6 (real SomaFM stream shows artist/title) not verified by the agent; needs a manual check.
- `nowPlaying` is only cleared by stop/station change (per existing reducer); it stays after a stream error until reconnect work in 05.
- User verified S6 manually on 2026-10-16. Resolved.
