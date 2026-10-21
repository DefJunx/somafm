# 03 Spectrum analyzer tap (rustfft)

Status: ready-for-agent
Blocked by:

## Goal
A PCM tap computes a 32-band log spectrum from the audio passing through `PcmSink::push`.

## Spec sections
- [Decisions](../spec.md#decisions-agreed-do-not-reopen) (Visualizer)
- [ADR 0003](../../../docs/adr/0003-audio-dsp-chain.md)

## Acceptance criteria
- [ ] `rustfft` added as a dependency.
- [ ] A pure `Spectrum` analyzer accepts interleaved samples, channels and rate; downmixes to mono; accumulates 2048-sample frames with a Hann window; returns 32 log-spaced band magnitudes in 0..1 with smoothed decay (fast attack, slow release).
- [ ] A `TapSink` wrapper implements `PcmSink`, forwards every `push` unchanged to the inner sink, and feeds the analyzer; forwarded samples are bit-identical (test with `NullSink`).
- [ ] Unit tests with the synthetic tone fixture (or a generated sine): the band containing the tone frequency is the maximum; silence gives all zeros; decay is monotonic after the tone stops; works for mono and stereo and for 44.1/48 kHz.
- [ ] Analysis cost is bounded: no allocation per push after warm-up (reuse buffers).
- [ ] `pnpm check` passes.
