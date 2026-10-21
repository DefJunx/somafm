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
- **Short sink queue while fading.** `RodioSink` normally queues about 1 s of decoded packets ahead of the playhead, so a ramp applied to newly decoded samples would only be heard that much later (and dropping the sink would cut unfaded audio). When the decode thread has fading on it opens the sink with a queue of about 6 packets (~150 ms). The same decision (`Option<Fade>`, read once when the decode thread starts) drives both the ramp and the sink's queue length, so a settings toggle mid-playback only applies to the next playback.
- **Cancel is deferred while fading.** `ChannelReader` normally fails a read as soon as `cancel` is set; with fading on it holds that off for the fade-out plus `FADE_OUT_SLACK` (200 ms) so the decode loop can finish the ramp from data already buffered. The decode loop itself gives up at the same deadline, so a stalled stream or sink after a cancel ends at the deadline and may cut abruptly (accepted).
- **Pause is stop.** The existing pause is the Stopped state and goes through the same cancel path, so it fades out like a stop; there is no separate pause fade.
- Quit waits for the fade-out plus a 100 ms margin at most.
