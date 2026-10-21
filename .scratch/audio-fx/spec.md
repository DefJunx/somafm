# Audio effects spec (0.4.0)

Smooth fades and a live Visualizer. Terms (Fade, Visualizer, Player screen) follow `GLOSSARY.md`. Decision record: [ADR 0003](../../docs/adr/0003-audio-dsp-chain.md). Settings toggles come from [auto-update ticket 01](../auto-update/issues/01-settings-popover-v2.md).

## Decisions (agreed, do not reopen)
- **Fades are sequential, not a true two-stream crossfade.** A per-sample gain ramp inside the decode loop (not stepped `set_volume` calls), multiplying the user volume; the saved volume is unchanged. Fixed durations: about 250 ms fade-out, about 600 ms fade-in.
- On cancel (stop or Station switch) the old decode thread fades out for about 250 ms before dropping its sink; the new sink fades in on its first audio. A brief natural overlap is accepted. Quit also fades, delaying exit by up to about 250 ms.
- Controlled by "Fade audio" (default on). When off, behaviour is exactly as today.
- **Visualizer**: about 32 log-spaced bands, bar spectrum, Player screen only, a strip about 48 px tall under the Now playing text, drawn on a canvas in the Theme teal accent.
- Computed in Rust: a tap wrapping `PcmSink` at `PcmSink::push`; downmix to mono, 2048-sample Hann window, `rustfft` (new dependency), 32 log bands with smoothed decay. Emitted at about 30 fps only while the Player screen is visible and playing; no events otherwise.
- Controlled by "Show visualizer" (default on; off when the OS prefers reduced motion).
- Unit-testable with the synthetic tone fixture (`src-tauri/tests/fixtures/tone.mp3`).

Source: grilling session with the user, 2026-10-21.

## Tickets
01 gain ramp -> 02 fades on start/stop/switch/quit -> 03 spectrum tap -> 04 spectrum events gating -> 05 Visualizer UI -> 06 release 0.4.0. 03 is independent of 01/02. 01 and 04 need the settings from auto-update 01.

Every ticket: test-first per the v1 test strategy; the gate is `pnpm check` (use `pnpm check:toolbox` on this host).

## Out of scope
- A true crossfade between two simultaneous streams.
- Configurable fade durations.
- The Visualizer anywhere other than the Player screen (Grid, NowPlayingBar, tray).
- Equalizer or other DSP.
- Audio-output tests (tests use `NullSink`).
