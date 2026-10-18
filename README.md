# somafm

An unofficial desktop player for [SomaFM](https://somafm.com), the listener-supported internet radio service. Built with Tauri, Svelte and Rust.

> **Disclaimer:** This is an independent project. It is not affiliated with, endorsed by, or sponsored by SomaFM. SomaFM is a listener-supported, commercial-free service, so please consider [supporting them](https://somafm.com).

## Features

- Station grid showing every SomaFM channel, with a player screen for the selected station
- Now-playing track information
- Favourite stations
- Automatic reconnect when a stream drops, with fallback between a station's mirror servers
- System tray with a now-playing label, play/stop toggle, favourites and Show/Hide
- Closing the window hides it to the tray and playback continues; quit from the tray menu or with Cmd/Ctrl+Q
- Media keys and OS now-playing overlay via [souvlaki](https://crates.io/crates/souvlaki) (MPRIS on Linux, SMTC on Windows, MPNowPlaying on macOS)
- Native Rust audio pipeline (HTTP, Symphonia MP3 decoding, rodio output); see [ADR 0001](docs/adr/0001-native-rust-audio.md)
- Persistent settings and window state

## Platform support

| Platform | Status |
| --- | --- |
| Linux | Build and install verified (Bazzite 44 KDE) |
| macOS | Build scripts provided, unverified |
| Windows | Build scripts provided, unverified |

Platform notes:

- **Linux:** the tray uses AppIndicator, which is menu-only (no click events), so use Show/Hide in the tray menu. KDE shows the tray natively; stock GNOME needs the [AppIndicator extension](https://extensions.gnome.org/extension/615/appindicator-support/).
- **macOS:** the dock icon stays visible while the window is hidden; clicking it reopens the window.
- **Windows:** the tray icon appears in the notification area.

## Install

There are no prebuilt releases; build from source. Full, platform-specific instructions (including the Linux toolbox build environment, runtime libraries and troubleshooting) are in [BUILD.md](BUILD.md).

| Platform | Build | Install |
| --- | --- | --- |
| Linux | `pnpm build:linux` | `./scripts/install-linux.sh` (installs to `~/.local`) |
| macOS | `pnpm build:macos` | Copy `somafm.app` to `/Applications` |
| Windows | `pnpm build:windows` | Run the generated NSIS installer |

## Build from source

Prerequisites: Rust stable ([rustup](https://rustup.rs/)), Node.js LTS and pnpm. On Linux you also need WebKitGTK and other development libraries; see [BUILD.md](BUILD.md).

```bash
pnpm install
pnpm tauri dev      # run with hot reload (on Linux: pnpm tauri:dev, which uses the toolbox)
pnpm check          # Rust fmt/clippy/tests, svelte-check and frontend tests
```

## Tech stack

- **Shell:** [Tauri 2](https://tauri.app) (tray icon, window state and store plugins)
- **Frontend:** Svelte 5, TypeScript, Vite, Vitest, [Open Props](https://open-props.style)
- **Backend:** Rust (edition 2024), tokio, reqwest (rustls), Symphonia, rodio, souvlaki

## Project structure

```
src/              Svelte frontend (views/, lib/ state and API wrappers)
src-tauri/src/    Rust backend: player/, feed, tray, settings, media_controls, commands
scripts/          install-linux.sh
docs/adr/         Architecture decision records
GLOSSARY.md       Domain terms (Station, Grid, Mirror, ...)
BUILD.md          Build and install guide
```

## Contributing

Issues and pull requests are welcome. Before opening a PR, run `pnpm check` and make sure it passes. Domain terms used in code and discussion are defined in [GLOSSARY.md](GLOSSARY.md); design decisions are recorded in [docs/adr](docs/adr).

## License

Released under the [MIT License](LICENSE). You are free to use, modify and distribute this software, provided the copyright notice and license text are retained (attribution to DefJunx).

## Acknowledgements

All stations and stream content belong to [SomaFM](https://somafm.com). If you enjoy the music, please [support them](https://somafm.com).
