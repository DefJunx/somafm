# 01 Per-sample gain ramp in the decode loop

Status: ready-for-agent
Blocked by: [auto-update 01](../../auto-update/issues/01-settings-popover-v2.md)

## Goal
The decode loop can apply a linear gain ramp to PCM samples, controlled by the "Fade audio" setting.

## Spec sections
- [Decisions](../spec.md#decisions-agreed-do-not-reopen) (Fades)
- [ADR 0003](../../../docs/adr/0003-audio-dsp-chain.md)

## Acceptance criteria
- [ ] A pure `Ramp` (target gain, duration in samples at the stream rate, current gain) applies per-sample gain to an interleaved `f32` buffer with the same gain across channels. Unit tests: ramp up/down endpoints, mid-ramp interruption continues from the current gain, buffer-boundary continuity, 0 duration = instant.
- [ ] `decode_stream` applies the ramp before `PcmSink::push`; the user volume still goes through `set_volume` and is not altered; the saved volume is not touched.
- [ ] With "Fade audio" off the sample stream is bit-identical to today (test with `NullSink` and the tone fixture).
- [ ] The decode thread reads the setting when it starts (changes apply to the next playback).
- [ ] `pnpm check` passes.
