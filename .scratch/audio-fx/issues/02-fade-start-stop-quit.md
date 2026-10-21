# 02 Fade in on start, fade out on stop, switch and quit

Status: ready-for-agent
Blocked by: 01

## Goal
Starting, stopping, switching Station and quitting use Fades.

## Spec sections
- [Decisions](../spec.md#decisions-agreed-do-not-reopen) (Fades)
- [ADR 0003](../../../docs/adr/0003-audio-dsp-chain.md)

## Acceptance criteria
- [ ] The first audio of a new sink fades in over about 600 ms (also after a reconnect to another Mirror).
- [ ] On cancel (stop or Station switch) the old decode thread ramps to zero over about 250 ms, then drops its sink; the new Station's sink fades in; brief overlap is accepted and not tested for exact timing.
- [ ] Quit triggers the fade-out and exits within about 250 ms plus a small margin; a hung decode thread cannot delay exit beyond a hard cap.
- [ ] Pause behaviour is unchanged unless the existing pause path already goes through cancel; document the choice in the report.
- [ ] Shell integration tests with stub servers + `NullSink`: sample amplitude rises on start, falls to zero before the sink drops on cancel, with fade off the old behaviour holds. Durations are injectable for fast tests.
- [ ] `pnpm check` passes.

## Manual verification (user)
1. Press play: audio eases in. Stop: eases out. Switch Station: old fades while new eases in.
2. Quit while playing: no click or pop, the app closes promptly.
3. Turn "Fade audio" off in Settings: behaviour is as before.
