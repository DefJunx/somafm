# 03 Play a station: .pls -> Mirror -> native decode, PlayerState reducer, player screen

Status: ready-for-agent
Blocked by: 02

## Goal
End-to-end tracer: click a Tile, resolve the `.pls`, stream and decode natively, drive it all through the pure `PlayerState` reducer with snapshot events, and show the Player screen and Grid active state.

## Spec sections
- [§3 Architecture](../spec.md#3-architecture-rust-vs-frontend) (PlayerState, `play`, `stop`, `set_volume`, `set_quality`, `get_player_state`, `player-state`)
- [§5 Playback](../spec.md#5-playback) (5.1, 5.2, 5.4, 5.6; first-connect pass only)
- [§6 UI](../spec.md#6-ui) (Tile click, Player screen, active ring + badge, bottom bar)

## Acceptance criteria
- [ ] Pure reducer `(state, event, now) -> (state, effects)` with table-driven tests (test-first) covering play, stop, connect success, first-connect Mirror pass, quality change, volume; shell executes effects; every change emits one full `player-state` snapshot.
- [ ] `.pls` parser (ordered Mirrors) and per-station Mirror-list cache; HTTP -> symphonia `MediaSource` adapter; MP3 and AAC-LC decode to rodio; null-sink test proves MP3 and AAC fixtures yield PCM; stub-server tests for `.pls` and stream.
- [ ] Clicking a Tile opens the Player screen and plays; Player screen has artwork, name, description, listeners, Stop/Play button, volume + mute, quality picker (MP3 / AAC 128, in-memory this slice), Back button.
- [ ] Back to Grid keeps audio: Active Tile has accent ring + equaliser badge; bottom now-playing bar shows Station with Stop/Play and a jump to the Player screen. Vitest covers these states.
- [ ] Minimal `connecting` and error ending (`error` after one failed pass) exist so the reducer is total; full error UI and reconnect are slice 05.

## Smoke checklist
S3, S4, S5. Shared Definition of Done in [§9 Testing & DoD](../spec.md#9-testing--the-check-command) applies.

## Notes
- Volume default 0.8 hard-coded here; persistence is slice 06.
- Playback must not depend on the webview (it must survive the window being hidden later).
- Generated tone fixtures only; no SomaFM bytes.
