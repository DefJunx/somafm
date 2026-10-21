# 05 Visualizer strip on the Player screen

Status: ready-for-agent
Blocked by: 04

## Goal
The Player screen shows the live bar spectrum.

## Spec sections
- [Decisions](../spec.md#decisions-agreed-do-not-reopen) (Visualizer)

## Acceptance criteria
- [ ] A `Visualizer.svelte` canvas about 48 px tall under the Now playing text draws 32 bars in the Theme teal accent token, scaled for device pixel ratio, sized responsively.
- [ ] The Player screen calls `set_visualizer_active(true)` on mount/visible and false on unmount, window hide and navigation to the Grid; it listens for `spectrum` only while active.
- [ ] Rendering is skipped (no canvas draw loop) when "Show visualizer" is off or when not playing; bars settle to flat when paused/stopped.
- [ ] The first-run default honours reduced motion via the settings from ticket 00; no extra motion logic here.
- [ ] Vitest with `mockIPC` and a mocked canvas context: activation calls, event-to-draw mapping, hidden when setting off.
- [ ] `pnpm check` passes.

## Manual verification (user)
1. Play a Station and open the Player screen: bars move with the music.
2. Go back to the Grid: CPU use drops, no events (check with devtools or logs).
3. Turn "Show visualizer" off: the strip disappears.
