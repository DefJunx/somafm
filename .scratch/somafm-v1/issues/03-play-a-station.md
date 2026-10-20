# 03 Play a station: .pls -> Mirror -> native decode, PlayerState reducer, player screen

Status: resolved
Blocked by: 02

## Goal
End-to-end tracer: click a Tile, resolve the `.pls`, stream and decode natively, drive it all through the pure `PlayerState` reducer with snapshot events, and show the Player screen and Grid active state.

## Spec sections
- [§3 Architecture](../spec.md#3-architecture-rust-vs-frontend) (PlayerState, `play`, `stop`, `set_volume`, `get_player_state`, `player-state`)
- [§5 Playback](../spec.md#5-playback) (5.1, 5.2, 5.4, 5.6; first-connect pass only)
- [§6 UI](../spec.md#6-ui) (Tile click, Player screen, active ring + badge, bottom bar)

## Acceptance criteria
- [x] Pure reducer `(state, event, now) -> (state, effects)` with table-driven tests (test-first) covering play, stop, connect success, first-connect Mirror pass, volume; shell executes effects; every change emits one full `player-state` snapshot.
- [x] `.pls` parser (ordered Mirrors) and per-station Mirror-list cache; HTTP -> symphonia `MediaSource` adapter; MP3 decode to rodio; null-sink test proves the MP3 fixture yields PCM; stub-server tests for `.pls` and stream.
- [x] Clicking a Tile opens the Player screen and plays; Player screen has artwork, name, description, listeners, Stop/Play button, volume + mute, Back button.
- [x] Back to Grid keeps audio: Active Tile has accent ring + equaliser badge; bottom now-playing bar shows Station with Stop/Play and a jump to the Player screen. Vitest covers these states.
- [x] Minimal `connecting` and error ending (`error` after one failed pass) exist so the reducer is total; full error UI and reconnect are slice 05.

## Smoke checklist
S3, S4. Shared Definition of Done in [§9 Testing & DoD](../spec.md#9-testing--the-check-command) applies.

## Notes
- Volume default 0.8 hard-coded here; persistence is slice 06.
- Playback must not depend on the webview (it must survive the window being hidden later).
- Generated tone fixtures only; no SomaFM bytes.

## Comments
- Automated checks pass (`pnpm check:toolbox`). Real audio output and smoke items S3-S4 were not run by the agent; verify by hand with `pnpm tauri dev`.
- Stall/first-byte timeout is not enforced yet (blocking client has no total timeout; connect timeout is 10 s). Belongs to slice 05.
- Mute is frontend-only (set_volume 0 / restore last volume); no separate mute command in the spec.
- Awaiting manual smoke check (S3-S4) by the user before marking resolved. Known leftover: a stalled pump thread stays blocked on its socket until the server closes (no audio device held); stall timeouts land in slice 05.

## Comments
- 2026-10-16: the quality picker, `set_quality` and AAC decode were removed from v1. SomaFM's `aac` streams are HE-AAC (MPEG-2 ADTS 0xFFF9, 22.05 kHz core) which symphonia cannot decode; see ADR 0001. v1 is MP3 only; smoke S5 is dropped.
- User verified S3-S4 manually on 2026-10-16 (MP3 only). Resolved.
