# Audio DSP in the decode loop: gain ramp and PCM tap

Fades and the Visualizer are implemented on the decoded PCM inside the Rust decode loop (`decode_stream` → `PcmSink::push`), not in the webview and not with stepped `set_volume` calls.

- **Fade**: a per-sample gain ramp applied inside the decode loop, multiplying the user volume (the saved volume is unchanged). Stepped `set_volume` calls are polled once per packet (~25 ms) and would be audibly stair-stepped.
- **Visualizer**: a tap wrapping `PcmSink` at `PcmSink::push` downmixes to mono and computes a 2048-sample Hann-windowed FFT (`rustfft`), reduced to ~32 log-spaced bands with smoothed decay. Spectrum events are emitted only while the Player screen is visible and playing.

## Why sequential fades, not a crossfade

A true crossfade needs two simultaneous decoded streams mixed into one output. Each mirror attempt owns its own `RodioSink`, and the old decode thread only notices a cancel within ~100 ms. We instead fade the old stream out (~250 ms) before its sink is dropped and fade the new one in (~600 ms) on its first audio; a brief natural overlap of the two sinks is accepted. This is far simpler and good enough for radio.

## Consequences

- Quit is delayed by up to ~250 ms so the fade-out can finish.
- Fade durations are fixed (not configurable).
- New dependency: `rustfft`.
- Both effects are pure functions over sample buffers, so they are unit-testable with the `NullSink` and the synthetic tone fixture; no audio output is needed in tests.
