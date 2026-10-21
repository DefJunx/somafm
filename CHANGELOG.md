# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-10-21

### Added

- Station history on the Player screen, listing recently played tracks for the current Station.
- Like tracks with a heart on the current track, on history rows and in the now-playing bar.
- Liked view listing liked tracks newest first, grouped by date (Today, Yesterday, This week, then by month).
- From the Liked view you can play the Station, copy a track, search it on Bandcamp, Discogs or MusicBrainz, or remove it.
- Removing a liked track can be undone, putting it back in its original place.
- Filter liked tracks by track or Station name.

## [0.1.1] - 2026-10-20

### Added

- Each release now includes a `SHA256SUMS` file for checking downloads.
- Release artifacts carry GitHub build provenance attestations, verifiable with `gh attestation verify`.
- README section on verifying downloads.

## [0.1.0] - 2026-10-20

### Added

- Station grid with feed fetch, cache, backoff refresh and skeleton tiles.
- Playback of SomaFM stations (MP3 via `.pls` mirrors) with a player screen and a persistent now-playing bar.
- Now-playing track titles from ICY `StreamTitle`.
- Reconnect/backoff state machine with timeouts, error UI and status badges.
- Settings store, favourites and window-state persistence.
- System tray menu with close-to-tray.
- Media keys and OS now-playing overlay.
- Dark theme with semantic tokens, focus/hover/disabled states and an inline icon set.
- Resizable window with responsive layouts and a grid/player transition.
- Linux install script, desktop template and per-OS build scripts.
- Release workflow building Linux, macOS and Windows artifacts.
- MIT license.

### Changed

- Streams are MP3 only; AAC and stream-quality selection are out of scope for v1.

### Fixed

- Streams in an unrecognised format now fail as unsupported instead of hanging in connecting.
- Tray menu refresh no longer deadlocks the main thread.
- Now-playing bar no longer scrolls away after a view transition.

[Unreleased]: https://github.com/DefJunx/somafm/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/DefJunx/somafm/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/DefJunx/somafm/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/DefJunx/somafm/releases/tag/v0.1.0
