# Native Rust audio engine instead of webview `<audio>`

Playback runs in the Tauri Rust backend (`reqwest` stream → `symphonia` decode → `rodio` output), not in an HTML5 `<audio>` element. Media keys, the system tray and playback that survives a hidden window are all v1 features, and they all need playback outside the webview. Native playback also lets us read ICY now-playing metadata inline and avoids depending on GStreamer codec packages on Linux.

## Consequences

- Symphonia decodes AAC-LC but not HE-AAC, so SomaFM's `aacp` (64k/32k) streams are not offered. The only stream qualities are MP3 and AAC 128. Restoring HE-AAC would mean an FFmpeg binding or `xaac-rs`.
- We own a small adapter that turns an HTTP byte stream into a symphonia `MediaSource`. The adapter also strips ICY metadata out of the audio bytes.
