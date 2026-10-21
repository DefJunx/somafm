# 04 Spectrum events, gated by visibility and playback

Status: resolved
Blocked by: 00, 03

## Goal
Rust emits spectrum events at about 30 fps only when something can show them.

## Spec sections
- [Decisions](../spec.md#decisions-agreed-do-not-reopen) (Visualizer)

## Acceptance criteria
- [x] A `set_visualizer_active(bool)` command lets the frontend say the Player screen is visible; events flow only when active AND playing AND "Show visualizer" is on AND the window is visible.
- [x] A `spectrum` event carries the 32 band values at about 30 fps (throttled from the tap, not per push); no events and no FFT work when gating is false. The gating decision is a pure function of those four inputs, unit-tested.
- [x] The tap is installed around the sink in the engine shell without changing the fade path; with the setting off the tap does no FFT work (always installed so enabling mid-stream works; user decision 2026-10-21).
- [x] Shell integration test (stub server + `NullSink`): events arrive while active and playing, stop on deactivate, pause, stop and setting off.
- [x] `pnpm check` passes.
