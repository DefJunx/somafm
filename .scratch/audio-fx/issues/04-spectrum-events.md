# 04 Spectrum events, gated by visibility and playback

Status: ready-for-agent
Blocked by: 03, [auto-update 01](../../auto-update/issues/01-settings-popover-v2.md)

## Goal
Rust emits spectrum events at about 30 fps only when something can show them.

## Spec sections
- [Decisions](../spec.md#decisions-agreed-do-not-reopen) (Visualizer)

## Acceptance criteria
- [ ] A `set_visualizer_active(bool)` command lets the frontend say the Player screen is visible; events flow only when active AND playing AND "Show visualizer" is on AND the window is visible.
- [ ] A `spectrum` event carries the 32 band values at about 30 fps (throttled from the tap, not per push); no events and no FFT work when gating is false. The gating decision is a pure function of those four inputs, unit-tested.
- [ ] The tap is installed around the sink in the engine shell without changing the fade path; with the setting off the tap is not installed.
- [ ] Shell integration test (stub server + `NullSink`): events arrive while active and playing, stop on deactivate, pause, stop and setting off.
- [ ] `pnpm check` passes.
