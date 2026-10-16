# 04 Now playing: ICY artist/title on the player and bar

Status: ready-for-agent
Blocked by: 03

## Goal
Parse ICY `StreamTitle` inline and show artist and title on the Player screen and the bottom bar.

## Spec sections
- [§5 Playback](../spec.md#5-playback) (5.1 ICY adapter, 5.3)
- [§3 Architecture](../spec.md#3-architecture-rust-vs-frontend) (`NowPlaying`, `player-state`)
- [§6 UI](../spec.md#6-ui) (Player screen, bottom bar)

## Acceptance criteria
- [ ] ICY framing in the stream adapter (`Icy-MetaData: 1`, `icy-metaint`) strips metadata from audio bytes; tested against a stub server emitting ICY-framed tone fixtures.
- [ ] `StreamTitle` parser table-tested (test-first): no separator, several separators, empty, non-UTF-8, stray quotes; split on the first " - " into artist/title; no separator -> `artist: None`.
- [ ] `nowPlaying` updates via `player-state`; kept across reconnects, cleared on `stop` and Station change.
- [ ] Player screen and bottom bar show `artist - title` (station name as artist when none); Vitest covers both shapes and the cleared state.

## Smoke checklist
S6. Shared Definition of Done in [§9 Testing & DoD](../spec.md#9-testing--the-check-command) applies.

## Notes
- Do not use feed `lastPlaying` for the Active station.
